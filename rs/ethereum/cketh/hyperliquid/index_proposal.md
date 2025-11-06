# Proposal to Install the ckHYPE Index Canister

Git hash: `5ecbd59c6c9f9f874d4340f9fbbd96af07aa2576`

New compressed Wasm hash: `3a6d39b5e94cdef5203bca62720e75a28cd071ff434d22b9746403ac7ae59614`

Target canister: `s3zol-vqaaa-aaaar-qacpa-cai`

---

## Motivation

This proposal install the mainnet ckHYPE index to the governance-controlled canister ID [`s3zol-vqaaa-aaaar-qacpa-cai`](https://dashboard.internetcomputer.org/canister/s3zol-vqaaa-aaaar-qacpa-cai) on subnet [`pzp6e-ekpqk-3c5x7-2h6so-njoeq-mt45d-h3h6c-q3mxf-vpeq5-fk5o7-yae`](https://dashboard.internetcomputer.org/subnet/pzp6e-ekpqk-3c5x7-2h6so-njoeq-mt45d-h3h6c-q3mxf-vpeq5-fk5o7-yae).

## Install args

```
git fetch
git checkout 5ecbd59c6c9f9f874d4340f9fbbd96af07aa2576
cd rs/rosetta-api/icrc1/index-ng
didc encode -d index-ng.did -t '(opt IndexArg)' '(opt variant { Init = record { ledger_id = principal "<CKHYPE_LEDGER_CANISTER_ID>" } })'
```

`<CKHYPE_LEDGER_CANISTER_ID>` is the ckHYPE ledger canister id.


## Wasm Verification

Verify that the hash of the gzipped WASM matches the proposed hash.

```
git fetch
git checkout 5ecbd59c6c9f9f874d4340f9fbbd96af07aa2576
./gitlab-ci/container/build-ic.sh -c
sha256sum ./artifacts/canisters/ic-icrc1-index-ng-u256.wasm.gz
```
