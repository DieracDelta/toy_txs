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
       dispute, 1, 1
       resolve, 1, 1
       dispute, 2, 2,
       chargeback, 2, 2,"#;

const DATA_1_EXPECTED_TXS: [Transaction; 9] = [
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
const DATA_1_EXPECTED: [&str; 3] = [
    "client,available,held,total,locked",
    "1,1.5000,0.0000,1.5000,false",
    "2,0.0000,0.0000,0.0000,true",
];

const DATA_3: &str = r#"
       type, client ,tx , amount
       withdrawal,  1,4,    1.5,,,,"#;

const DATA_3_EXPECTED: [&str; 2] = [
    "client,available,held,total,locked",
    "1,0.0000,0.0000,0.0000,false",
];

const DATA_4: &str = r#"
       type, client ,tx , amount
       deposit,  1, 1,    5
       withdrawal,  1, 2,    3
       dispute, 1, 2,
       dispute, 1, 1,"#;

const DATA_4_EXPECTED: [&str; 2] = [
    "client,available,held,total,locked",
    "1,0.0000,2.0000,2.0000,false",
];

const DATA_5: &str = r#"
       type, client ,tx , amount
       deposit,  1, 1,    5
       withdrawal,  1, 2,    3
       dispute, 1, 2,
       chargeback, 1, 2,
       deposit,  1, 1,    5"#;

const DATA_5_EXPECTED: [&str; 2] = [
    "client,available,held,total,locked",
    "1,5.0000,0.0000,5.0000,true",
];

const DATA_6: &str = r#"
       type, client ,tx , amount
       deposit,  1, 1,    5
       withdrawal,  1, 2,    3
       dispute, 1, 2,
       resolve, 1, 2,
       deposit,  1, 1,    5"#;

const DATA_6_EXPECTED: [&str; 2] = [
    "client,available,held,total,locked",
    "1,7.0000,0.0000,7.0000,false",
];

const DATA_7: &str = r#"
       type, client ,tx , amount
       deposit,  1, 1,    5
       withdrawal,  1, 2,    3
       dispute, 1, 1,
       resolve, 1, 2,
       resolve, 1, 1,
       deposit,  1, 1,    5"#;

const DATA_7_EXPECTED: [&str; 2] = [
    "client,available,held,total,locked",
    "1,7.0000,0.0000,7.0000,false",
];

const DATA_8: &str = r#"
       type, client ,tx , amount
       deposit,  1, 1,    5
       withdrawal,  1, 2,    3
       dispute, 1, 1,
       chargeback, 1, 1,
       deposit,  1, 1,    5
       withdrawal,  1, 8,    3"#;

const DATA_8_EXPECTED: [&str; 2] = [
    "client,available,held,total,locked",
    "1,-3.0000,0.0000,-3.0000,true",
];

/// helper function to test that the processed account's serialized results matches the account's actual value
fn test_data(data: &str, data_expected: Vec<&str>) -> Result<()> {
    let mut rdr = ReaderBuilder::new()
        .trim(Trim::All)
        .flexible(true)
        .delimiter(b',')
        .from_reader(data.as_bytes());
    let mut accounts = Accounts::new();
    for result in rdr.deserialize() {
        accounts.process_transaction(&result?);
    }
    let mut serialized_result = vec![];
    accounts.serialize_to_writer(&mut serialized_result)?;
    let serialized_result_utf8 = String::from_utf8(serialized_result)?;
    let lines_expected: HashSet<String> = data_expected.iter().map(|x| x.to_string()).collect();
    assert_eq!(
        lines_expected,
        serialized_result_utf8
            .lines()
            .map(|x| x.to_string())
            .collect::<HashSet<String>>()
    );
    Ok(())
}

/// tests that (1) transactions after resolve tx do effect final result
/// and        (2) deposit returns held funds to avail funds
#[test]
pub fn test_deposit_resolve() -> Result<()> {
    test_data(DATA_7, DATA_7_EXPECTED.to_vec())
}

/// tests that chargeback works with deposit
#[test]
pub fn test_deposit_chargeback() -> Result<()> {
    test_data(DATA_8, DATA_8_EXPECTED.to_vec())
}

/// tests that (1) transactions after chargeback do not effect final result
/// and        (2) chargeback returns withdrawn funds
#[test]
pub fn test_withdrawal_resolve() -> Result<()> {
    test_data(DATA_6, DATA_6_EXPECTED.to_vec())
}

/// tests that (1) transactions after chargeback do not effect final result
/// and        (2) chargeback returns withdrawn funds
#[test]
pub fn test_withdrawal_chargeback() -> Result<()> {
    test_data(DATA_5, DATA_5_EXPECTED.to_vec())
}

/// tests that withdrawal and deposit that happen consecutively is handled correctly
#[test]
pub fn test_disputes() -> Result<()> {
    test_data(DATA_4, DATA_4_EXPECTED.to_vec())
}

/// tests withdrawal without positive balance
#[test]
pub fn test_single_withdrawal() -> Result<()> {
    test_data(DATA_3, DATA_3_EXPECTED.to_vec())
}

/// check that deserialization works as expected
#[test]
pub fn test_deserialize() -> Result<()> {
    let data = DATA_1;
    let mut rdr = ReaderBuilder::new()
        .trim(Trim::All)
        .flexible(true)
        .delimiter(b',')
        .from_reader(data.as_bytes());
    let mut expected_results = DATA_1_EXPECTED_TXS.to_vec();
    expected_results.reverse();
    for result in rdr.deserialize() {
        let record: Transaction = result?;
        let expected_record = expected_results.pop().unwrap();
        //println!("parsed record: {:?}", record);
        //println!("expected record: {:?}\n", expected_record);
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
        .flexible(true)
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
        .flexible(true)
        .delimiter(b',')
        .from_reader(data.as_bytes());
    let mut accounts = Accounts::new();
    for result in rdr.deserialize() {
        let finished = result?;
        accounts.process_transaction(&finished);
        //println!("tx: {:?}\naccount: {:?}\n\n", &finished, accounts);
    }
    let mut expected_accounts = Accounts::new();
    let mut expected_hm_account_1 = HashMap::new();
    let mut expected_hm_account_2 = HashMap::new();
    expected_hm_account_1.insert(DATA_1_EXPECTED_TXS[0].tx_id, DATA_1_EXPECTED_TXS[0]);
    expected_hm_account_1.insert(DATA_1_EXPECTED_TXS[2].tx_id, DATA_1_EXPECTED_TXS[2]);
    expected_hm_account_1.insert(DATA_1_EXPECTED_TXS[3].tx_id, DATA_1_EXPECTED_TXS[3]);
    expected_hm_account_2.insert(DATA_1_EXPECTED_TXS[1].tx_id, DATA_1_EXPECTED_TXS[1]);
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
    test_data(DATA_1, DATA_1_EXPECTED.to_vec())
}
