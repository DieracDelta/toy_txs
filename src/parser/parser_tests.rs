use crate::parser::accounts::{Account, Accounts};
use crate::parser::transactions::{FloatingPoint, FloatingPointWrap, Transaction, TransactionType};
use anyhow::Result;
use csv::{ReaderBuilder, Trim};
use fixed_macro::fixed;
use std::collections::{HashMap, HashSet};

const DATA_1: &str = r#"
       type, client ,tx , amount
       deposit, 1,1, 1.0
       deposit,   2,2, 2.0
       deposit, 1,3,    2.0
       withdrawal,  1,4,    1.5
       withdrawal, 2, 5,   3.0
       dispute, 1, 1,
       resolve, 1, 1,
       dispute, 2, 2,
       chargeback, 2, 2,"#;
const DATA_1_EXPECTED: [Transaction; 9] = [
    Transaction {
        transaction_type: TransactionType::Deposit,
        client_id: 1,
        tx_id: 1,
        amount: Some(FloatingPointWrap(fixed!(1.0: I113F15))),
    },
    Transaction {
        transaction_type: TransactionType::Deposit,
        client_id: 2,
        tx_id: 2,
        amount: Some(FloatingPointWrap(fixed!(2.0: I113F15))),
    },
    Transaction {
        transaction_type: TransactionType::Deposit,
        client_id: 1,
        tx_id: 3,
        amount: Some(FloatingPointWrap(fixed!(2.0: I113F15))),
    },
    Transaction {
        transaction_type: TransactionType::Withdrawal,
        client_id: 1,
        tx_id: 4,
        amount: Some(FloatingPointWrap(fixed!(1.5: I113F15))),
    },
    Transaction {
        transaction_type: TransactionType::Withdrawal,
        client_id: 2,
        tx_id: 5,
        amount: Some(FloatingPointWrap(fixed!(3.0: I113F15))),
    },
    Transaction {
        transaction_type: TransactionType::Dispute,
        client_id: 1,
        tx_id: 1,
        amount: None,
    },
    Transaction {
        transaction_type: TransactionType::Resolve,
        client_id: 1,
        tx_id: 1,
        amount: None,
    },
    Transaction {
        transaction_type: TransactionType::Dispute,
        client_id: 2,
        tx_id: 2,
        amount: None,
    },
    Transaction {
        transaction_type: TransactionType::Chargeback,
        client_id: 2,
        tx_id: 2,
        amount: None,
    },
];
const DATA_2: [&str; 3] = [
    "client,available,held,total,locked",
    "1,1.5000,0.0000,1.5000,false",
    "2,0.0000,0.0000,0.0000,true",
];

/// check that deserialization works as expected
#[test]
pub fn test_deserialize() -> Result<()> {
    let data = DATA_1;
    let mut rdr = ReaderBuilder::new()
        .trim(Trim::All)
        .delimiter(b',')
        .from_reader(data.as_bytes());
    let mut expected_results = DATA_1_EXPECTED.to_vec();
    expected_results.reverse();
    for result in rdr.deserialize() {
        let record: Transaction = result?;
        let expected_record = expected_results.pop().unwrap();
        println!("parsed record: {:?}", record);
        println!("expected record: {:?}\n", expected_record);
        assert!(record.check_state());
        assert_eq!(record, expected_record);
    }
    Ok(())
}

/// check that empty edgecase is hit
#[test]
pub fn test_empty() -> Result<()> {
    let data = r#"
    type,client,txt,amount "#;
    let mut rdr = ReaderBuilder::new()
        .trim(Trim::All)
        .delimiter(b',')
        .from_reader(data.as_bytes());
    assert_eq!(
        rdr.deserialize()
            .collect::<Vec<Result<Transaction, csv::Error>>>()
            .len(),
        0
    );
    Ok(())
}

/// check the transaction processing step works as expected
#[test]
pub fn test_process_txs() -> Result<()> {
    let data = DATA_1;
    let mut rdr = ReaderBuilder::new()
        .trim(Trim::All)
        .delimiter(b',')
        .from_reader(data.as_bytes());
    let mut accounts = Accounts::new();
    for result in rdr.deserialize() {
        let finished = result?;
        accounts.process_transaction(&finished);
        println!("tx: {:?}\naccount: {:?}\n\n", &finished, accounts);
    }
    let mut expected_accounts = Accounts::new();
    let mut expected_hm_account_1 = HashMap::new();
    let mut expected_hm_account_2 = HashMap::new();
    expected_hm_account_1.insert(DATA_1_EXPECTED[0].tx_id, DATA_1_EXPECTED[0]);
    expected_hm_account_1.insert(DATA_1_EXPECTED[2].tx_id, DATA_1_EXPECTED[2]);
    expected_hm_account_1.insert(DATA_1_EXPECTED[3].tx_id, DATA_1_EXPECTED[3]);
    expected_hm_account_2.insert(DATA_1_EXPECTED[1].tx_id, DATA_1_EXPECTED[1]);
    expected_accounts.state.insert(
        1,
        Account {
            client_id: 1,
            avail_bal: FloatingPoint::from_num(1.5),
            held_bal: FloatingPoint::from_num(0),
            total_bal: FloatingPoint::from_num(1.5),
            locked: false,
            transactions: expected_hm_account_1,
            disputes: HashSet::new(),
        },
    );
    expected_accounts.state.insert(
        2,
        Account {
            client_id: 2,
            avail_bal: FloatingPoint::from_num(0),
            held_bal: FloatingPoint::from_num(0),
            total_bal: FloatingPoint::from_num(0),
            locked: true,
            transactions: expected_hm_account_2,
            disputes: HashSet::new(),
        },
    );
    assert_eq!(accounts, expected_accounts);

    Ok(())
}

/// check serialization works as expected
#[test]
pub fn test_serialize() -> Result<()> {
    let data = DATA_1;
    let mut rdr = ReaderBuilder::new()
        .trim(Trim::All)
        .delimiter(b',')
        .from_reader(data.as_bytes());
    let mut accounts = Accounts::new();
    for result in rdr.deserialize() {
        accounts.process_transaction(&result?);
    }
    let mut serialized_result = vec![];
    accounts.serialize_to_writer(&mut serialized_result)?;
    let serialized_result_utf8 = String::from_utf8(serialized_result)?;
    let lines_expected: HashSet<String> = DATA_2.iter().map(|x| x.to_string()).collect();
    assert_eq!(
        serialized_result_utf8.lines().collect::<Vec<&str>>().len(),
        lines_expected.len()
    );
    for actual_line in serialized_result_utf8.lines() {
        if !lines_expected.contains(actual_line) {
            assert!(false);
        }
    }

    Ok(())
}
