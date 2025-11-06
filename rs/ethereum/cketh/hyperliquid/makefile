ifneq (,$(wildcard .env))
  include .env
  export
endif

ZIG_GLOBAL_CACHE_DIR := $(abspath .zig-cache/global)
ZIG_LOCAL_CACHE_DIR  := $(abspath .zig-cache/local)
export ZIG_GLOBAL_CACHE_DIR ZIG_LOCAL_CACHE_DIR

SHELL := /bin/bash
.ONESHELL:
.SHELLFLAGS := -eu -o pipefail -c
.PHONY: init all \
  deploy-ckhype-minter-init deploy-ckhype-minter-upgrade deploy-orchestrator-init deploy-ckhype-ledger \
  cast-deposit-eth cast-approve cast-deposit-erc20 orchestrator-add-erc20 orchestrator-upgrade-hashes print-hashes \
  estimate-eth-fee estimate-erc20-fee approve-cketh-amount approve-ckerc20-amount approve-cketh-fee \
  withdraw-eth withdraw-erc20 status-by-sender status-eth-id

# Env: HELPER_ADDR, HYPEREVM_RPC_URL, PRIVATE_KEY should be defined in .env or export before running make
# ERC20_ADDR defaults to sample token
ERC20_ADDR ?= 0xa9056c15938f9aff34CD497c722Ce33dB0C2fD57
CHAIN_ID_ERC20 ?= 998

# Withdraw params (override via env or .env)
RECIP ?= 0xYourEthAddress
AMOUNT_WEI ?= 10_000_000_000_000_000
ERC_AMOUNT ?=
CKERC20_LEDGER ?=
CKETH_FEE_LIMIT ?=10_000_000_000_000_000
WITHDRAWAL_ID ?=

# Wasm paths (optional, used for hash-pinned orchestrator upgrade)
LEDGER_WASM_GZ ?= ../../../../bazel-bin/rs/ledger_suite/icrc1/ledger/ledger_canister_u256.wasm.gz
INDEX_WASM_GZ  ?= ../../../../bazel-bin/rs/ledger_suite/icrc1/index-ng/index_ng_canister_u256.wasm.gz
ARCHIVE_WASM_GZ?= ../../../../bazel-bin/rs/ledger_suite/icrc1/archive/archive_canister_u256.wasm.gz

# IDs
EVM_RPC_ID       := $(shell dfx canister id evm_rpc)
ORCH_ID          := $(shell dfx canister id orchestrator || true)
MINTER_ID        := $(shell dfx canister id ckhype_minter)
LEDGER_ID        := $(shell dfx canister id ckhype_ledger)
ARCHIVE_CANISTER := $(shell dfx canister id ckhype_archive)

# Hashes
LEDGER_HASH := $(shell [ -f "$(LEDGER_WASM_GZ)" ] && shasum -a 256 "$(LEDGER_WASM_GZ)" | awk '{print $$1}')
INDEX_HASH  := $(shell [ -f "$(INDEX_WASM_GZ)" ]  && shasum -a 256 "$(INDEX_WASM_GZ)"  | awk '{print $$1}')
ARCHIVE_HASH:= $(shell [ -f "$(ARCHIVE_WASM_GZ)" ]&& shasum -a 256 "$(ARCHIVE_WASM_GZ)"| awk '{print $$1}')

# Create canisters once; sentinel prevents re-running
init: canisters

canisters:
	# Create if missing
	dfx canister id evm_rpc        >/dev/null 2>&1 || dfx canister create evm_rpc
	dfx canister id orchestrator   >/dev/null 2>&1 || dfx canister create orchestrator
	dfx canister id ckhype_minter  >/dev/null 2>&1 || dfx canister create ckhype_minter
	dfx canister id ckhype_ledger  >/dev/null 2>&1 || dfx canister create ckhype_ledger
	dfx canister id ckhype_archive >/dev/null 2>&1 || dfx canister create ckhype_archive

deploy-evm-rpc:
	@echo "Deploy evm_rpc"
	dfx deploy evm_rpc

deploy-ckhype-minter-init:
	@echo "Deploy ckhype_minter (InitArg)"
	dfx deploy ckhype_minter --argument '(variant { InitArg = record { \
	  ethereum_network = variant { Hyperliquid }; \
	  ecdsa_key_name = "dfx_test_key"; \
	  ethereum_contract_address = opt "0x0000000000000000000000000000000000000001"; \
	  ledger_id = principal "$(LEDGER_ID)"; \
	  ethereum_block_height = variant { Finalized }; \
	  minimum_withdrawal_amount = 10000000000000000; \
	  next_transaction_nonce = 0; \
	  last_scraped_block_number = 36914385; \
	} })'

deploy-ckhype-minter-upgrade:
	@echo "Upgrade ckhype_minter (UpgradeArg)"
	dfx deploy ckhype_minter --mode upgrade --upgrade-unchanged --argument '(variant { \
	  UpgradeArg = record { \
	    evm_rpc_id = opt principal "$(EVM_RPC_ID)"; \
	    ledger_suite_orchestrator_id = opt principal "$(ORCH_ID)"; \
	    deposit_with_subaccount_helper_contract_address = opt "$(HELPER_ADDR)"; \
		erc20_helper_contract_address = opt "$(HELPER_ADDR)"; \
	    last_erc20_scraped_block_number = opt 36914385; \
	  } \
	})'

deploy-orchestrator-init:
	@echo "Deploy orchestrator (InitArg)"
	dfx deploy orchestrator --argument '(variant { InitArg = record { \
	  more_controller_ids = vec {}; \
	  minter_id = opt principal "$(MINTER_ID)"; \
	  cycles_management = opt record { \
	    cycles_for_ledger_creation = 2000000000000; \
	    cycles_for_archive_creation = 1000000000000; \
	    cycles_for_index_creation = 1000000000000; \
	    cycles_top_up_increment = 500000000000 \
	  } \
	} })'

deploy-ckhype-ledger:
	@echo "Deploy ckhype_ledger"
	dfx deploy ckhype_ledger --argument '(variant { Init = record { \
	  minting_account = record { owner = principal "$(MINTER_ID)" }; \
	  feature_flags = opt record { icrc2 = true }; \
	  decimals = opt 18; \
	  max_memo_length = opt 80; \
	  transfer_fee = 10000000000; \
	  token_symbol = "ckHYPE"; \
	  token_name = "Chain key Hyperliquid"; \
	  metadata = vec {}; \
	  initial_balances = vec {}; \
	  archive_options = record { \
	    num_blocks_to_archive = 1000; \
	    trigger_threshold = 2000; \
	    max_message_size_bytes = null; \
	    cycles_for_archive_creation = opt 1000000000000; \
	    node_max_memory_size_bytes = opt 3221225472; \
	    controller_id = principal "$(ARCHIVE_CANISTER)" \
	  } \
	} })'

cast-deposit-eth:
	@echo "depositEth"
	cast send "$(HELPER_ADDR)" \
	  'depositEth(bytes32,bytes32)' \
	  0x1d44d3ef615f749bba2e1e6e777c8660bec189026410cdb055fd1f82b4020000 \
	  0x0000000000000000000000000000000000000000000000000000000000000000 \
	  --value 100000000000000000 \
	  --rpc-url "$(HYPEREVM_RPC_URL)" \
	  --private-key "$(PRIVATE_KEY)"

cast-approve:
	@echo "approve ERC20 -> helper"
	cast send "$(ERC20_ADDR)" \
	  'approve(address,uint256)' \
	  "$(HELPER_ADDR)" \
	  1000000000000000000000 \
	  --rpc-url "$(HYPEREVM_RPC_URL)" \
	  --private-key "$(PRIVATE_KEY)"

cast-deposit-erc20:
	@echo "depositErc20"
	cast send "$(HELPER_ADDR)" \
	  'depositErc20(address,uint256,bytes32,bytes32)' \
	  "$(ERC20_ADDR)" \
	  100000000000000 \
	  0x1d44d3ef615f749bba2e1e6e777c8660bec189026410cdb055fd1f82b4020000 \
	  0x0000000000000000000000000000000000000000000000000000000000000000 \
	  --rpc-url "$(HYPEREVM_RPC_URL)" \
	  --private-key "$(PRIVATE_KEY)"

orchestrator-add-erc20:
	@echo "AddErc20Arg -> orchestrator"
	dfx deploy orchestrator --mode upgrade --upgrade-unchanged --argument '(variant { \
	  AddErc20Arg = record { \
	    contract = record { \
	      chain_id = $(CHAIN_ID_ERC20):nat; \
	      address  = "$(ERC20_ADDR)"; \
	    }; \
	    ledger_init_arg = record { \
	      decimals        = 18; \
	      transfer_fee    = 4000:nat; \
	      token_symbol    = "ckPURR"; \
	      token_name      = "Chain Key Purr"; \
	      token_logo      = ""; \
	    }; \
	  } \
	})'

orchestrator-upgrade-hashes:
	@if [ -z "$(LEDGER_HASH)" ] || [ -z "$(INDEX_HASH)" ] || [ -z "$(ARCHIVE_HASH)" ]; then \
	  echo "Missing WASM hashes. Run print-hashes."; \
	  exit 1; \
	fi
	dfx deploy orchestrator --mode upgrade --upgrade-unchanged --argument '(variant { UpgradeArg = record { \
	  git_commit_hash = opt "f8296b2ebf95db2f75e845d68f3fcfb4478dfcd7"; \
	  ledger_compressed_wasm_hash = opt "$(LEDGER_HASH)"; \
	  index_compressed_wasm_hash  = opt "$(INDEX_HASH)"; \
	  archive_compressed_wasm_hash = opt "$(ARCHIVE_HASH)"; \
	  manage_ledger_suites = null \
	} })'

print-hashes:
	@echo "LEDGER_HASH=$(LEDGER_HASH)"
	@echo "INDEX_HASH=$(INDEX_HASH)"
	@echo "ARCHIVE_HASH=$(ARCHIVE_HASH)"

# ===== Withdraw helpers =====
estimate-eth-fee:
	@MINTER_ID=$$(dfx canister id ckhype_minter); \
	dfx canister call "$$MINTER_ID" eip_1559_transaction_price '(null)'

estimate-erc20-fee:
	@CKERC20_LEDGER="$${CKERC20_LEDGER}"; \
	if [ -z "$$CKERC20_LEDGER" ]; then \
	  CKERC20_LEDGER=$$(dfx canister call orchestrator canister_ids "(record { chain_id = $(CHAIN_ID_ERC20) : nat; address = \"$(ERC20_ADDR)\" })" | tr -d '\n' | sed -n 's/.*ledger = opt principal "\([^"]*\)".*/\1/p'); \
	fi; \
	test -n "$$CKERC20_LEDGER" || (echo "Could not resolve CKERC20_LEDGER from orchestrator; set CKERC20_LEDGER or ensure token is added." && exit 1); \
	MINTER_ID=$$(dfx canister id ckhype_minter); \
	dfx canister call "$$MINTER_ID" eip_1559_transaction_price "(opt record { ckerc20_ledger_id = principal \"$$CKERC20_LEDGER\" })"

approve-cketh-amount:
	@test -n "$(AMOUNT_WEI)" || (echo "Set AMOUNT_WEI"; exit 1)
	@MINTER_ID=$$(dfx canister id ckhype_minter); \
	CKETH_LEDGER=$$(dfx canister id ckhype_ledger); \
	AMT=$$(echo "$(AMOUNT_WEI)" | tr -d '_'); \
	dfx canister call "$$CKETH_LEDGER" icrc2_approve "(record { \
	  spender = record { owner = principal \"$$MINTER_ID\"; subaccount = null }; \
	  amount = $$AMT : nat; \
	  fee = null; from_subaccount = null; created_at_time = null; \
	  expected_allowance = null; expires_at = null; memo = null \
	})"

approve-ckerc20-amount:
	@test -n "$(ERC_AMOUNT)" || (echo "Set ERC_AMOUNT"; exit 1)
	@CKERC20_LEDGER="$${CKERC20_LEDGER}"; \
	if [ -z "$$CKERC20_LEDGER" ]; then \
	  CKERC20_LEDGER=$$(dfx canister call orchestrator canister_ids "(record { chain_id = $(CHAIN_ID_ERC20) : nat; address = \"$(ERC20_ADDR)\" })" | tr -d '\n' | sed -n 's/.*ledger = opt principal "\([^"]*\)".*/\1/p'); \
	fi; \
	echo "Using CKERC20_LEDGER=$$CKERC20_LEDGER"; \
	test -n "$$CKERC20_LEDGER" || (echo "Could not resolve CKERC20_LEDGER from orchestrator; set CKERC20_LEDGER or ensure token is added." && exit 1); \
	MINTER_ID=$$(dfx canister id ckhype_minter); \
	AMT=$$(echo "$(ERC_AMOUNT)" | tr -d '_'); \
	dfx canister call "$$CKERC20_LEDGER" icrc2_approve "(record { \
	  spender = record { owner = principal \"$$MINTER_ID\"; subaccount = null }; \
	  amount = $$AMT : nat; \
	  fee = null; from_subaccount = null; created_at_time = null; \
	  expected_allowance = null; expires_at = null; memo = null \
	})"

approve-cketh-fee:
	@test -n "$(CKETH_FEE_LIMIT)" || (echo "Set CKETH_FEE_LIMIT"; exit 1)
	@MINTER_ID=$$(dfx canister id ckhype_minter); \
	CKETH_LEDGER=$$(dfx canister id ckhype_ledger); \
	FEE=$$(echo "$(CKETH_FEE_LIMIT)" | tr -d '_'); \
	dfx canister call "$$CKETH_LEDGER" icrc2_approve "(record { \
	  spender = record { owner = principal \"$$MINTER_ID\"; subaccount = null }; \
	  amount = $$FEE : nat; \
	  fee = null; from_subaccount = null; created_at_time = null; \
	  expected_allowance = null; expires_at = null; memo = null \
	})"

withdraw-eth:
	@test -n "$(RECIP)" || (echo "Set RECIP"; exit 1)
	@test -n "$(AMOUNT_WEI)" || (echo "Set AMOUNT_WEI"; exit 1)
	@MINTER_ID=$$(dfx canister id ckhype_minter); \
	AMT=$$(echo "$(AMOUNT_WEI)" | tr -d '_'); \
	dfx canister call "$$MINTER_ID" withdraw_eth "(record { recipient = \"$(RECIP)\"; amount = $$AMT : nat; from_subaccount = null })"

withdraw-erc20:
	@test -n "$(RECIP)" || (echo "Set RECIP"; exit 1)
	@test -n "$(ERC_AMOUNT)" || (echo "Set ERC_AMOUNT"; exit 1)
	@CKERC20_LEDGER="$${CKERC20_LEDGER}"; \
	if [ -z "$$CKERC20_LEDGER" ]; then \
	  CKERC20_LEDGER=$$(dfx canister call orchestrator canister_ids "(record { chain_id = $(CHAIN_ID_ERC20) : nat; address = \"$(ERC20_ADDR)\" })" | tr -d '\n' | sed -n 's/.*ledger = opt principal "\([^"]*\)".*/\1/p'); \
	fi; \
	echo "Using CKERC20_LEDGER=$$CKERC20_LEDGER"; \
	test -n "$$CKERC20_LEDGER" || (echo "Could not resolve CKERC20_LEDGER from orchestrator; set CKERC20_LEDGER or ensure token is added." && exit 1); \
	MINTER_ID=$$(dfx canister id ckhype_minter); \
	AMT=$$(echo "$(ERC_AMOUNT)" | tr -d '_'); \
	echo "Sending $$AMT to $$RECIP at $$MINTER_ID"; \
	dfx canister call "$$MINTER_ID" withdraw_erc20 "(record { \
	  amount = $$AMT : nat; \
	  ckerc20_ledger_id = principal \"$$CKERC20_LEDGER\"; \
	  recipient = \"$$RECIP\"; \
	  from_cketh_subaccount = null; \
	  from_ckerc20_subaccount = null \
	})"

balance:
	@PRINCIPAL=$$(dfx identity get-principal); \
	CKETH_LEDGER=$$(dfx canister id ckhype_ledger); \
	echo "ckETH balance for $$PRINCIPAL:"; \
	dfx canister call "$$CKETH_LEDGER" icrc1_balance_of "(record { owner = principal \"$$PRINCIPAL\"; subaccount = null })"

balance-erc20:
	@CKERC20_LEDGER="$${CKERC20_LEDGER}"; \
	if [ -z "$$CKERC20_LEDGER" ]; then \
	  CKERC20_LEDGER=$$(dfx canister call orchestrator canister_ids "(record { chain_id = $(CHAIN_ID_ERC20) : nat; address = \"$(ERC20_ADDR)\" })" | tr -d '\n' | sed -n 's/.*ledger = opt principal "\([^"]*\)".*/\1/p'); \
	fi; \
	test -n "$$CKERC20_LEDGER" || (echo "Could not resolve CKERC20_LEDGER from orchestrator; set CKERC20_LEDGER or ensure token is added." && exit 1); \
	PRINCIPAL=$$(dfx identity get-principal); \
	echo "ERC20 balance for $$PRINCIPAL on $$CKERC20_LEDGER:"; \
	dfx canister call "$$CKERC20_LEDGER" icrc1_balance_of "(record { owner = principal \"$$PRINCIPAL\"; subaccount = null })"


# Get minter address from on-chain helper
get-minter-address:
	@test -n "$(HELPER_ADDR)" || (echo "Set HELPER_ADDR"; exit 1)
	@test -n "$(HYPEREVM_RPC_URL)" || (echo "Set HYPEREVM_RPC_URL"; exit 1)
	@echo "Helper reports minter address:"
	cast call "$(HELPER_ADDR)" "getMinterAddress()(address)" --rpc-url "$(HYPEREVM_RPC_URL)"


# Cross-check against IC minter view
get-minter-address-ic:
	@MINTER_ID=$$(dfx canister id ckhype_minter); \
	echo "IC minter address:"; \
	dfx canister call "$$MINTER_ID" minter_address

# Deploy CkDeposit helper contract
# Requires: PRIVATE_KEY, HYPEREVM_RPC_URL
# Uses the IC minter address as constructor arg

deploy-helper:
	@test -n "$(HYPEREVM_RPC_URL)" || (echo "Set HYPEREVM_RPC_URL"; exit 1)
	@test -n "$(PRIVATE_KEY)" || (echo "Set PRIVATE_KEY"; exit 1)
	@MINTER_ID=$$(dfx canister id ckhype_minter); \
	OUT=$$(dfx canister call $$MINTER_ID minter_address | tr -d '\n\r'); \
	MINTER_HEX=$$(echo "$$OUT" | grep -oE '0x[0-9a-fA-F]{40}' | head -n1); \
	test -n "$$MINTER_HEX" || (echo "Failed to obtain minter ETH address from IC (got: $$OUT)"; exit 1); \
	test -n "$$MINTER_HEX" || (echo "Failed to obtain minter ETH address from IC"; exit 1); \
	echo "Deploying CkDeposit with minter $$MINTER_HEX"; \
	forge create  --broadcast --root ../minter --private-key "$(PRIVATE_KEY)" --rpc-url "$(HYPEREVM_RPC_URL)" \
		DepositHelperWithSubaccount.sol:CkDeposit --constructor-args $$MINTER_HEX 