# usage
- cargo run --release -- transactions.csv > accounts.csv

# what is this?
A toy transaction ledger.

# feature set
Input data set is a CSV (see tests for example inputs) including the following types of transactions:

- Deposit: deposit an amount of money to an account
- Withdraw: if there are enough funds to withdraw, withdraw funds from account. Otherwise this is a noop.
- Dispute: dispute one of the transactions on an account. Currently this is restricted to credit. Intuitively a client would never dispute receiving credt (who would dispute receiving money?). However, it is not clear what the behavior should be in these other cases, so these are noops.
- Resolve: resolution to a dispute. The money is released to the client.
- Chargeback: second type of resolution to a dispute. The money is removed from client's account.

# testing:
- Integration test cases are included to check that the serialization/deserialization is done correctly.

# interesting edge cases:
- What happens if there is overflow? This application uses a 128 signed number with 15 bits of precision. This gives us slightly more than 4 decimal places of precision (1/2^15 = 0.00003). This rep allows us to check if any arithmetic operation causes overflow. If it does, the transaction is cancelled and becomes a noop.
- What happens if there is underflow? No division is happening so this is not a concern.
- What happens if a Dispute transaction is disputed? Currently this is no-oped as a server error. It's not clear what should happen when the transaction type is not a deposit. This would require the addition of a "confirmed" withdrawal.
- What happens if input CSV format is malformed? The program will error.

# performance:
It is hard to handle large data sets currently, since all deposits must be tracked in case there is a dispute. In the future, an addition of a database (or really any non-volatile storage) would be the morally correct solution to avoid large ram usage while maintaining efficiency.

# input:

csv:
{
  type: string
  client: u16
  tx: u32
  amount: float with four digits past the decimal of precision
}

# output
csv:
{
    client: u16,
    available: float with four digits past the decimal of precision,
    held: float with four digits past the decimal of precision,
    total: float with four digits past the decimal of precision,
    locked: bool
}
