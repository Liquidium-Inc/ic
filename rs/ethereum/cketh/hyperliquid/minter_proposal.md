# Proposal to Install the ckHYPE Minter Canister

Git hash: `5ecbd59c6c9f9f874d4340f9fbbd96af07aa2576`

New compressed Wasm hash: `e0167373ddd503c06a93faa2dac2d8da8118894a2552fc811186e31d5c49f27e`

Target canister: <CKHYPE_MINTER_CANISTER_ID>

---

## Motivation

This proposal install the mainnet ckHYPE minter to the governance-controlled canister ID [`<CKHYPE_MINTER_CANISTER_ID>`](https://dashboard.internetcomputer.org/canister/<CKHYPE_MINTER_CANISTER_ID>) on subnet [`pzp6e-ekpqk-3c5x7-2h6so-njoeq-mt45d-h3h6c-q3mxf-vpeq5-fk5o7-yae`](https://dashboard.internetcomputer.org/subnet/pzp6e-ekpqk-3c5x7-2h6so-njoeq-mt45d-h3h6c-q3mxf-vpeq5-fk5o7-yae).

## Install args

```
git fetch
git checkout 5ecbd59c6c9f9f874d4340f9fbbd96af07aa2576
cd rs/ethereum/cketh/minter
didc encode -d cketh_minter.did -t '(MinterArg)' '(
  variant {
    InitArg = record {
      ethereum_network = variant { Custom = record { chain_id = 998; rpc_url = "<HYPEREVM_RPC_URL>" } };
      ecdsa_key_name = "key_1";
      ethereum_contract_address = opt "<HELPER_CONTRACT_ADDRESS>";
      ledger_id = principal "<CKHYPE_LEDGER_CANISTER_ID>";
      ethereum_block_height = variant { Finalized };
      minimum_withdrawal_amount = 30_000_000_000_000_000;
      next_transaction_nonce = 0;
      last_scraped_block_number = <HYPEREVM_START_BLOCK>
    }
  }
)'
```

* <CKHYPE_LEDGER_CANISTER_ID> is the ckHYPE ledger canister.
* <HELPER_CONTRACT_ADDRESS> is the deployed HyperEVM helper contract address used for deposits and withdrawals.
* <HYPEREVM_START_BLOCK> is the HyperEVM block number at which the helper contract was deployed (start scanning from here).
* The minimum withdrawal amount of 0.03 HYPE (30_000_000_000_000_000 base units) is a rough equivalent of 60 USD as of November 30, 2023.

## Wasm Verification

Verify that the hash of the gzipped WASM matches the proposed hash.

```
git fetch
git checkout 5ecbd59c6c9f9f874d4340f9fbbd96af07aa2576
./gitlab-ci/container/build-ic.sh -c
sha256sum ./artifacts/canisters/ic-cketh-minter.wasm.gz
```
