# Drift

Due to some issues, drift comes with a typescript component.
https://github.com/drift-labs/protocol-v2/issues/891#issuecomment-2030929726

## Configure

Set the following environment variables in your `.env` file.

```dotenv
DRIFT_ADDRESS=
DRIFT_PRIVATE_KEY=

DRIFT_JS_PATH=/home/work/cefi/js/drift
SOLANA_COMPUTE_UNIT_PRICE=5000

```

Note that SOLANA_COMPUTE_UNIT_PRICE should be 5000-20000 to make sure orders are successfully executed
