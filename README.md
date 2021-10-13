# Usage
- cargo run --release -- transactions.csv > accounts.csv

# What is this?
A toy transaction ledger.

# Feature set
Input data set is a CSV (see tests for example inputs) including the following types of transactions:

- Deposit: deposit an amount of money to an account
- Withdraw: if there are enough available funds to withdraw, withdraw funds from account. Otherwise this is a noop.
- Dispute: dispute one of the transactions on an account. 
- Resolve: first type of resolution to a dispute. The money is released to the client.
- Chargeback: second type of resolution to a dispute. The money is removed/refunded from/to client's account.

# Testing:
- Integration test cases are included to check that the serialization/deserialization is done correctly. Property based testing is a better way to approach this in the future.

# Interesting edge cases:
- What happens if there is overflow? This application uses a fixed-point 128 signed floating point number with 15 bits of precision. This gives us slightly more than 4 decimal places of precision (1/2^15 = 0.00003). This rep allows us to check if any arithmetic operation causes overflow. If it does, the transaction is cancelled and becomes a noop.
- What happens if there is underflow? No division is happening so this is not a concern.
- What happens if input CSV format is malformed? The program will error.

# Error handling
I'm using `anyhow` to propagate errors through out of `main`. However for now, I've opted for the strawman approach to not error unless there is an issue parsing the input. The remainder of the time, transactions are just be ignored if they do not fit within the spec. In the future I'd like to add a custom error type for each point of failure in a transaction, use `thiserror` for detailed descriptions, and and explicitly output errors into a text file (but still not fail visibly to stdout).

# Performance:
It is hard to handle large data sets currently, since all deposits must be tracked in case there is a dispute. In the future, the addition of a database (or really any non-volatile storage) would be the morally correct solution to avoid large ram usage while maintaining speed.

# Input:

```
csv:
{
  type: string
  client: u16
  tx: u32
  amount: float with four digits past the decimal of precision
}
```

# output
```
csv:
{
    client: u16,
    available: float with four digits past the decimal of precision,
    held: float with four digits past the decimal of precision,
    total: float with four digits past the decimal of precision,
    locked: bool
}
```
