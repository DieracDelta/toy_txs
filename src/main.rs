mod parser;
use anyhow::{bail, Result};
use csv::{ReaderBuilder, Trim};
use parser::{accounts::Accounts, transactions::Transaction};
use std::{env, io};

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        bail!("Wrong number of arguments. Expected exactly one. Usage: `./transactions input.csv`");
    }
    let mut rdr = ReaderBuilder::new()
        .trim(Trim::All)
        .delimiter(b',')
        .from_path(&args[1])?;
    let mut accounts = Accounts::new();
    // if there is an error deserializing, fail.
    rdr.deserialize()
        .try_for_each(|ele: Result<Transaction, csv::Error>| -> Result<()> {
            let tx = ele?;
            assert!(tx.check_state());
            accounts.process_transaction(&tx);
            Ok(())
        })?;
    accounts.serialize_to_writer(io::stdout())?;

    Ok(())
}
