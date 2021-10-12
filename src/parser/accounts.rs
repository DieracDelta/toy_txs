use crate::parser::transactions::{
    deserialize_floating_point, FloatingPoint, Transaction, TransactionType,
};
use anyhow::Result;
use csv::WriterBuilder;
use serde::{self, Deserialize, Serialize, Serializer};
use std::collections::{HashMap, HashSet};
use std::io::Write;

// Assumptions: only deposits can be disputed
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct Account {
    #[serde(rename = "client")]
    pub client_id: u16,
    #[serde(
        rename = "available",
        serialize_with = "serialize_floating_point",
        deserialize_with = "deserialize_floating_point"
    )]
    pub avail_bal: FloatingPoint,
    #[serde(
        rename = "held",
        serialize_with = "serialize_floating_point",
        deserialize_with = "deserialize_floating_point"
    )]
    pub held_bal: FloatingPoint,
    #[serde(
        rename = "total",
        serialize_with = "serialize_floating_point",
        deserialize_with = "deserialize_floating_point"
    )]
    pub total_bal: FloatingPoint,
    pub locked: bool,
    // list of transactions. id -> Transaction
    // Needed in the case of a dispute
    #[serde(skip_serializing)]
    pub transactions: HashMap<u32, Transaction>,
    // set of unresolved disputes
    #[serde(skip_serializing)]
    pub disputes: HashSet<u32>,
}

impl Account {
    fn new(client_id: u16) -> Self {
        Account {
            client_id,
            avail_bal: FloatingPoint::from_num(0),
            held_bal: FloatingPoint::from_num(0),
            total_bal: FloatingPoint::from_num(0),
            locked: false,
            transactions: HashMap::new(),
            disputes: HashSet::new(),
        }
    }
}

/// Represents a set of accounts. Internal rep is a map from account id to account metadata
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Accounts {
    pub state: HashMap<u16, Account>,
}

impl Accounts {
    /// creates new set of Accounts
    pub fn new() -> Self {
        Accounts {
            state: HashMap::new(),
        }
    }

    /// serialize state and prints to stdout
    /// overwrites any existing data
    /// throws error if unable to serialize
    pub fn serialize_to_writer(&self, w: impl Write) -> Result<()> {
        // better to be explicit in case library defaults change
        let mut wtr = WriterBuilder::new()
            .delimiter(b',')
            .has_headers(false)
            .from_writer(w);
        wtr.write_record(vec!["client", "available", "held", "total", "locked"])?;
        for (_, account) in self.state.iter() {
            wtr.serialize(account)?;
        }
        wtr.flush()?;
        Ok(())
    }

    /// mutates `self` to reflect transaction `t`
    /// specification:
    /// - if an account is frozen, this function is a noop.
    /// - Deposit: adds amount to the account's total balance and available balance
    /// - Withdrawal: subtracts its amount from the account's total balance and available balance
    /// - Dispute: if the disputed tx exists and is a deposit, move the amount
    ///   from the available balance to the held balance. All other disputed transaction types are a noop.
    /// - Resolve: money is returned from the held balance to the avail balance
    /// - Chargeback: money is removed from the held balance and total balance.
    pub fn process_transaction(&mut self, t: &Transaction) {
        let account = self
            .state
            .entry(t.client_id)
            .or_insert_with(|| Account::new(t.client_id));
        if account.locked {
            return;
        }
        match t.transaction_type {
            TransactionType::Deposit | TransactionType::Withdrawal => {
                // we check the state during parsing.
                // This can never be empty
                let amount = t.amount.unwrap();
                let sign = t.transaction_type.get_sign();
                // balance doesn't go negative or become infinite; otherwise noop.
                let new_total_bal = amount
                    .0
                    .checked_mul(sign)
                    .and_then(|x| x.checked_add(account.total_bal));
                let new_avail_bal = amount
                    .0
                    .checked_mul(sign)
                    .and_then(|x| x.checked_add(account.avail_bal));
                if new_total_bal.is_some()
                    && new_avail_bal.is_some()
                    && new_total_bal.unwrap() >= FloatingPoint::from_num(0)
                    && new_avail_bal.unwrap() >= FloatingPoint::from_num(0)
                {
                    account.total_bal = new_total_bal.unwrap();
                    account.avail_bal = new_avail_bal.unwrap();
                    // will not be overwriting because tx ids are unique
                    // Could limit to just deposits for now.
                    account.transactions.insert(t.tx_id, *t);
                } else {
                    println!("skipping deposit {:?}", t);
                }
            }
            TransactionType::Dispute => {
                if let Some(disputed_tx) = account.transactions.get(&t.tx_id) {
                    // Only deposits make sense to be disputed; everything else is a noop.
                    // If the transaction is a withdrawal, then the money is already missing.
                    // One way to deal with this is to refund the account total balance and place
                    // funds on hold. I didn't implement this since it directly violates the spec.
                    if disputed_tx.transaction_type == TransactionType::Deposit {
                        if let Some(disputed_amount) = disputed_tx.amount {
                            let new_avail_bal = account.avail_bal.checked_sub(disputed_amount.0);
                            let new_held_bal = account.held_bal.checked_add(disputed_amount.0);
                            if new_held_bal.is_some() && new_avail_bal.is_some() {
                                account.avail_bal = new_avail_bal.unwrap();
                                account.held_bal = new_held_bal.unwrap();
                                account.disputes.insert(disputed_tx.tx_id);
                            }
                        }
                    }
                }
            }
            TransactionType::Resolve => {
                if let Some(disputed_tx) = account.transactions.get(&t.tx_id) {
                    // should always be true in order for it to be marked as disputed
                    if account.disputes.contains(&disputed_tx.tx_id) {
                        if let Some(disputed_amount) = disputed_tx.amount {
                            let new_held_bal = account.held_bal.checked_sub(disputed_amount.0);
                            let new_avail_bal = account.avail_bal.checked_add(disputed_amount.0);
                            if new_avail_bal.is_some() && new_held_bal.is_some() {
                                account.held_bal = new_held_bal.unwrap();
                                account.avail_bal = new_avail_bal.unwrap();
                                account.disputes.remove(&disputed_tx.tx_id);
                            }
                        }
                    }
                }
            }
            TransactionType::Chargeback => {
                if let Some(disputed_tx) = account.transactions.get(&t.tx_id) {
                    // should always be true in order for it to be marked as disputed
                    if account.disputes.contains(&disputed_tx.tx_id) {
                        if let Some(disputed_amount) = disputed_tx.amount {
                            let new_held_bal = account.held_bal.checked_sub(disputed_amount.0);
                            let new_total_bal = account.total_bal.checked_sub(disputed_amount.0);
                            if new_total_bal.is_some() && new_held_bal.is_some() {
                                account.held_bal = new_held_bal.unwrap();
                                account.total_bal = new_total_bal.unwrap();
                                account.disputes.remove(&disputed_tx.tx_id);
                                account.locked = true;
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Function to serialize floating point to 4 digits of precision.
///
fn serialize_floating_point<S>(f: &FloatingPoint, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&format!("{:.4}", f))
}
