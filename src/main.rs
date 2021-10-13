mod parser;
use anyhow::{bail, Result};
use csv::{ReaderBuilder, Trim};
use parser::{accounts::Accounts, transactions::Transaction};
use std::fs::File;
use std::io::BufReader;
use std::{env, io};

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        bail!("Wrong number of arguments. Expected exactly one. Usage: `./transactions input.csv`");
    }
    let f = File::open(&args[1])?;

    let mut rdr = ReaderBuilder::new()
        .trim(Trim::All)
        .delimiter(b',')
        .flexible(true)
        .from_reader(BufReader::new(f));
    let mut accounts = Accounts::new();
    let mut raw_record = csv::ByteRecord::new();
    let headers = rdr.byte_headers()?.clone();
    // if there is an error deserializing, fail.
    while rdr.read_byte_record(&mut raw_record)? {
        let tx: Transaction = raw_record.deserialize(Some(&headers))?;
        assert!(tx.check_state());
        accounts.process_transaction(&tx);
    }
    accounts.serialize_to_writer(io::stdout())?;

    Ok(())
}
