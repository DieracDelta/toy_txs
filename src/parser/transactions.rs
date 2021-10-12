#![allow(clippy::unnecessary_unwrap)]
use anyhow::Result;
use fixed::{types::extra::U15, FixedI128};
use serde::{
    self,
    de::{Error, Unexpected, Visitor},
    Deserialize, Deserializer, Serialize,
};
use std::fmt;

pub type FloatingPoint = FixedI128<U15>;

/// represents the type of transaction.
/// Currently there are five supported transaction types.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TransactionType {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}

impl TransactionType {
    /// The sign of the transaction
    /// Conceptually deposits "add" money to an account, so the sign is positive.
    /// Withdrawals "remove" money to an account, so the sign is negative.
    /// The remainder of transaction types are noops.
    pub fn get_sign(&self) -> FloatingPoint {
        match self {
            TransactionType::Deposit => FloatingPoint::from_num(1),
            TransactionType::Withdrawal => FloatingPoint::from_num(-1),
            // everything else is a noop
            _ => FloatingPoint::from_num(-1),
        }
    }
}

/// HACK to get custom serde deserializer to work
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Eq, PartialEq)]
pub struct FloatingPointWrap(
    #[serde(deserialize_with = "deserialize_floating_point")] pub FloatingPoint,
);

/// Transaction metadata
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Eq, PartialEq)]
pub struct Transaction {
    #[serde(rename = "type")]
    pub transaction_type: TransactionType,
    #[serde(rename = "client")]
    pub client_id: u16,
    #[serde(rename = "tx")]
    pub tx_id: u32,
    #[serde(rename = "amount")]
    pub amount: Option<FloatingPointWrap>,
}

impl Transaction {
    /// Checks that the state makes sense for the type of transaction.
    /// Specifically, only withdrawals and deposits can have an amount.
    pub fn check_state(&self) -> bool {
        match self.transaction_type {
            TransactionType::Deposit | TransactionType::Withdrawal => self.amount.is_some(),
            TransactionType::Dispute | TransactionType::Resolve | TransactionType::Chargeback => {
                self.amount.is_none()
            }
        }
    }
}

pub fn deserialize_floating_point<'de, D>(deserializer: D) -> Result<FloatingPoint, D::Error>
where
    D: Deserializer<'de>,
{
    deserializer.deserialize_str(FloatingPointVisitor)
}

struct FloatingPointVisitor;
impl<'de> Visitor<'de> for FloatingPointVisitor {
    type Value = FloatingPoint;
    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a string representation of a floating point (up to 4 decimal places)")
    }
    fn visit_str<E>(self, value: &str) -> Result<FloatingPoint, E>
    where
        E: Error,
    {
        value
            .parse::<f64>()
            .map(FloatingPoint::from_num)
            .map_err(|_err| {
                E::invalid_value(Unexpected::Str(value), &"a string representation of a f64")
            })
    }
}
