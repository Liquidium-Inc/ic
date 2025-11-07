ifneq (,$(wildcard .env))
  include .env
  export
endif

# Caches for zig (used by some IC builds)
ZIG_GLOBAL_CACHE_DIR := $(abspath .zig-cache/global)
ZIG_LOCAL_CACHE_DIR  := $(abspath .zig-cache/local)
export ZIG_GLOBAL_CACHE_DIR ZIG_LOCAL_CACHE_DIR

SHELL := /bin/bash
.ONESHELL:
.SHELLFLAGS := -eu -o pipefail -c

# -----------------------------------------------------------------------------
# Configuration
# -----------------------------------------------------------------------------

# External env expected (can be set in .env):
#   HELPER_ADDR, HYPEREVM_RPC_URL, PRIVATE_KEY
# ERC20 defaults
BTC_ERC20_ADDR  ?= 0x9FDBdA0A5e284c32744D2f17Ee5c74B284993463
USDC_ERC20_ADDR ?= 0xb88339CB7199b77E23DB6E890353E22632Ba630f
USDT_ERC20_ADDR ?= 0xB8CE59FC3717ada4C02eaDF9682A9e934F625ebb
CHAIN_ID_ERC20  ?= 999

# Default large approval amount for convenience (override with AMOUNT_MAX)
AMOUNT_MAX       ?= 1_000_000_000_000_000_000_000_000

# Withdraw params (override via env or .env)
RECIP           ?= 0xYourEthAddress
AMOUNT_WEI      ?= 10_000_000_000_000_000
ERC_AMOUNT      ?=
CKERC20_LEDGER  ?=
CKETH_FEE_LIMIT ?= 10_000_000_000_000_000
WITHDRAWAL_ID   ?=

# Wasm paths (optional, used for hash-pinned orchestrator upgrade)
LEDGER_WASM_GZ  ?= ../../../../bazel-bin/rs/ledger_suite/icrc1/ledger/ledger_canister_u256.wasm.gz
INDEX_WASM_GZ   ?= ../../../../bazel-bin/rs/ledger_suite/icrc1/index-ng/index_ng_canister_u256.wasm.gz
ARCHIVE_WASM_GZ ?= ../../../../bazel-bin/rs/ledger_suite/icrc1/archive/archive_canister_u256.wasm.gz

# Canister IDs
EVM_RPC_ID       := $(shell dfx canister id evm_rpc)
ORCH_ID          := $(shell dfx canister id ckhype_orchestrator || true)
MINTER_ID        := $(shell dfx canister id ckhype_minter)
LEDGER_ID        := $(shell dfx canister id ckhype_ledger)
ARCHIVE_CANISTER := $(shell dfx canister id ckhype_archive)

# Hashes (only computed if files exist)
LEDGER_HASH  := $(shell [ -f "$(LEDGER_WASM_GZ)" ]  && shasum -a 256 "$(LEDGER_WASM_GZ)"  | awk '{print $$1}')
INDEX_HASH   := $(shell [ -f "$(INDEX_WASM_GZ)" ]   && shasum -a 256 "$(INDEX_WASM_GZ)"   | awk '{print $$1}')
ARCHIVE_HASH := $(shell [ -f "$(ARCHIVE_WASM_GZ)" ] && shasum -a 256 "$(ARCHIVE_WASM_GZ)" | awk '{print $$1}')

# -----------------------------------------------------------------------------
# Reusable helpers and generic targets
# -----------------------------------------------------------------------------

# Resolve CKERC20_LEDGER for a given ERC20 address if not provided
# Usage: $(call RESOLVE_CKERC20_LEDGER,0xTokenAddress)
define RESOLVE_CKERC20_LEDGER
CKERC20_LEDGER="$$CKERC20_LEDGER";
if [ -z "$$CKERC20_LEDGER" ]; then \
  CKERC20_LEDGER=$$(dfx canister call ckhype_orchestrator canister_ids "(record { chain_id = $(CHAIN_ID_ERC20) : nat; address = \"$(1)\" })" | tr -d '\n' | sed -n 's/.*ledger = opt principal \"\([^\"]*\)\".*/\1/p'); \
fi; \
echo "Using CKERC20_LEDGER=$$CKERC20_LEDGER"; \
 test -n "$$CKERC20_LEDGER" || (echo "Could not resolve CKERC20_LEDGER from orchestrator; set CKERC20_LEDGER or ensure token is added." && exit 1);
endef

# Approve any ERC20 amount for the minter
approve-erc20-amount:
	@test -n "$(ERC_AMOUNT)" || (echo "Set ERC_AMOUNT"; exit 1)
	@$(call RESOLVE_CKERC20_LEDGER,$(ERC20_ADDR))
	@MINTER_ID=$$(dfx canister id ckhype_minter); \
	AMT=$$(echo "$(ERC_AMOUNT)" | tr -d '_'); \
	dfx canister call "$$CKERC20_LEDGER" icrc2_approve "(record { \
	  spender = record { owner = principal \"$$MINTER_ID\"; subaccount = null }; \
	  amount = $$AMT : nat; \
	  fee = null; from_subaccount = null; created_at_time = null; \
	  expected_allowance = null; expires_at = null; memo = null \
	})"

# Approve a large default amount for any ERC20 (convenience)
approve-erc20-max:
	@$(MAKE) approve-erc20-amount ERC20_ADDR=$(ERC20_ADDR) ERC_AMOUNT=$(AMOUNT_MAX)

# Coin-specific convenience wrappers
approve-btc-amount:
	@$(MAKE) approve-erc20-amount ERC20_ADDR=$(BTC_ERC20_ADDR) ERC_AMOUNT=$(ERC_AMOUNT)

approve-usdc-amount:
	@$(MAKE) approve-erc20-amount ERC20_ADDR=$(USDC_ERC20_ADDR) ERC_AMOUNT=$(ERC_AMOUNT)

approve-usdt-amount:
	@$(MAKE) approve-erc20-amount ERC20_ADDR=$(USDT_ERC20_ADDR) ERC_AMOUNT=$(ERC_AMOUNT)

approve-btc-max:
	@$(MAKE) approve-erc20-amount ERC20_ADDR=$(BTC_ERC20_ADDR) ERC_AMOUNT=$(AMOUNT_MAX)

approve-usdc-max:
	@$(MAKE) approve-erc20-amount ERC20_ADDR=$(USDC_ERC20_ADDR) ERC_AMOUNT=$(AMOUNT_MAX)

approve-usdt-max:
	@$(MAKE) approve-erc20-amount ERC20_ADDR=$(USDT_ERC20_ADDR) ERC_AMOUNT=$(AMOUNT_MAX)

# Batch convenience: approve max for common tokens
approve-all-max:
	@$(MAKE) approve-btc-max
	@$(MAKE) approve-usdc-max
	@$(MAKE) approve-usdt-max

# Withdraw any ERC20 through the minter
withdraw-erc20:
	@test -n "$(RECIP)" || (echo "Set RECIP"; exit 1)
	@test -n "$(ERC_AMOUNT)" || (echo "Set ERC_AMOUNT"; exit 1)
	@$(call RESOLVE_CKERC20_LEDGER,$(ERC20_ADDR))
	@MINTER_ID=$$(dfx canister id ckhype_minter); \
	AMT=$$(echo "$(ERC_AMOUNT)" | tr -d '_'); \
	echo "Sending $$AMT to $(RECIP) via $$MINTER_ID"; \
	dfx canister call "$$MINTER_ID" withdraw_erc20 "(record { \
	  amount = $$AMT : nat; \
	  ckerc20_ledger_id = principal \"$$CKERC20_LEDGER\"; \
	  recipient = \"$(RECIP)\"; \
	  from_cketh_subaccount = null; \
	  from_ckerc20_subaccount = null \
	})"

# Query balance for any ERC20 ledger
balance-erc20-any:
	@$(call RESOLVE_CKERC20_LEDGER,$(ERC20_ADDR))
	@PRINCIPAL=$$(dfx identity get-principal); \
	echo "ERC20 balance for $$PRINCIPAL on $$CKERC20_LEDGER:"; \
	dfx canister call "$$CKERC20_LEDGER" icrc1_balance_of "(record { owner = principal \"$$PRINCIPAL\"; subaccount = null })"

# -----------------------------------------------------------------------------
# Project targets
# -----------------------------------------------------------------------------
.PHONY: init canisters \
  deploy-evm-rpc deploy-ckhype-minter-init deploy-ckhype-minter-upgrade deploy-orchestrator-init deploy-ckhype-ledger \
  cast-deposit-eth cast-approve cast-deposit-erc20 orchestrator-add-erc20 orchestrator-upgrade-hashes print-hashes \
  estimate-eth-fee estimate-erc20-fee approve-cketh-amount approve-erc20-amount approve-cketh-fee \
  withdraw-eth withdraw-erc20 withdraw-btc withdraw-usdc balance balance-erc20 balance-erc20-any \
  get-minter-address get-minter-address-ic deploy-helper orchestrator-add-liqBtc orchestrator-add-liqUSDC

# Create canisters once; sentinel prevents re-running
init: canisters

canisters:
	# Create if missing
	# dfx canister id evm_rpc           >/dev/null 2>&1 || dfx canister create evm_rpc
	dfx canister id ckhype_orchestrator >/dev/null 2>&1 || dfx canister create ckhype_orchestrator
	dfx canister id ckhype_minter       >/dev/null 2>&1 || dfx canister create ckhype_minter
	dfx canister id ckhype_ledger       >/dev/null 2>&1 || dfx canister create ckhype_ledger
	dfx canister id ckhype_archive      >/dev/null 2>&1 || dfx canister create ckhype_archive

# -------------------- Deploy --------------------

deploy-evm-rpc:
	@echo "Deploy evm_rpc"
	dfx deploy evm_rpc

deploy-ckhype-minter-init:
	@echo "Deploy ckhype_minter (InitArg)"
	dfx deploy ckhype_minter --argument '(variant { InitArg = record { \
	  ethereum_network = variant { Hyperliquid }; \
	  ecdsa_key_name = "key_1"; \
	  ethereum_contract_address = opt "0x0000000000000000000000000000000000000001"; \
	  ledger_id = principal "$(LEDGER_ID)"; \
	  ethereum_block_height = variant { Finalized }; \
	  minimum_withdrawal_amount = 10000000000000000; \
	  next_transaction_nonce = 0; \
	  last_scraped_block_number = ; \
	} })'

deploy-ckhype-minter-upgrade:
	@echo "Upgrade ckhype_minter (UpgradeArg)"
	dfx deploy ckhype_minter --mode upgrade --upgrade-unchanged --argument '(variant { \
	  UpgradeArg = record { \
	    evm_rpc_id = opt principal "$(EVM_RPC_ID)"; \
	    ledger_suite_orchestrator_id = opt principal "$(ORCH_ID)"; \
	    deposit_with_subaccount_helper_contract_address = opt "$(HELPER_ADDR)"; \
		erc20_helper_contract_address = opt "$(HELPER_ADDR)"; \
	    last_erc20_scraped_block_number = opt 18572299; \
	  } \
	})'

deploy-orchestrator-init:
	@echo "Deploy orchestrator (InitArg)"
	dfx deploy ckhype_orchestrator --argument '(variant { InitArg = record { \
	  more_controller_ids = vec { principal "p42pg-fke2p-xwcx3-uto5c-4htoo-56imy-f6yge-qezaq-zwyfl-7i7qk-2ae" }; \
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
	  archive_options = record { \
	    num_blocks_to_archive = 1000; \
	    trigger_threshold = 2000; \
	    max_message_size_bytes = null; \
	    cycles_for_archive_creation = opt 1000000000000; \
	    node_max_memory_size_bytes = opt 3221225472; \
	    controller_id = principal "$(ARCHIVE_CANISTER)" \
	  } \
	} })'

# -------------------- Cast helper calls --------------------

cast-deposit-eth:
	@echo "depositEth"
	cast send "$(HELPER_ADDR)" \
	  'depositEth(bytes32,bytes32)' \
	  0x1d44d3ef615f749bba2e1e6e777c8660bec189026410cdb055fd1f82b4020000 \
	  0x0000000000000000000000000000000000000000000000000000000000000000 \
	  --value 10000000000000000 \
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

# Generic EVM approve with adjustable amount (defaults to AMOUNT_MAX if APPROVE_AMOUNT unset)
cast-approve-erc20:
	@test -n "$(ERC20_ADDR)" || (echo "Set ERC20_ADDR"; exit 1)
	@test -n "$(HELPER_ADDR)" || (echo "Set HELPER_ADDR"; exit 1)
	@test -n "$(HYPEREVM_RPC_URL)" || (echo "Set HYPEREVM_RPC_URL"; exit 1)
	@test -n "$(PRIVATE_KEY)" || (echo "Set PRIVATE_KEY"; exit 1)
	@AMT=$$( if [ -n "$(APPROVE_AMOUNT)" ]; then echo "$(APPROVE_AMOUNT)"; else echo "$(AMOUNT_MAX)"; fi | tr -d '_' ); \
	 echo "Approving $$AMT for $(ERC20_ADDR) -> $(HELPER_ADDR)"; \
	 cast send "$(ERC20_ADDR)" \
	  'approve(address,uint256)' \
	  "$(HELPER_ADDR)" \
	  $$AMT \
	  --rpc-url "$(HYPEREVM_RPC_URL)" \
	  --private-key "$(PRIVATE_KEY)"

# Coin-specific wrappers (use APPROVE_AMOUNT=... or default to AMOUNT_MAX)
cast-approve-btc:
	@$(MAKE) cast-approve-erc20 ERC20_ADDR=$(BTC_ERC20_ADDR) APPROVE_AMOUNT=$(APPROVE_AMOUNT)

cast-approve-usdc:
	@$(MAKE) cast-approve-erc20 ERC20_ADDR=$(USDC_ERC20_ADDR) APPROVE_AMOUNT=$(APPROVE_AMOUNT)

cast-approve-usdt:
	@$(MAKE) cast-approve-erc20 ERC20_ADDR=$(USDT_ERC20_ADDR) APPROVE_AMOUNT=$(APPROVE_AMOUNT)

# Max wrappers
cast-approve-btc-max:
	@$(MAKE) cast-approve-erc20 ERC20_ADDR=$(BTC_ERC20_ADDR) APPROVE_AMOUNT=$(AMOUNT_MAX)

cast-approve-usdc-max:
	@$(MAKE) cast-approve-erc20 ERC20_ADDR=$(USDC_ERC20_ADDR) APPROVE_AMOUNT=$(AMOUNT_MAX)

cast-approve-usdt-max:
	@$(MAKE) cast-approve-erc20 ERC20_ADDR=$(USDT_ERC20_ADDR) APPROVE_AMOUNT=$(AMOUNT_MAX)

# Batch convenience
cast-approve-all-max:
	@$(MAKE) cast-approve-btc-max
	@$(MAKE) cast-approve-usdc-max
	@$(MAKE) cast-approve-usdt-max

cast-deposit-erc20:
	@echo "depositErc20"
	@test -n "$(ERC20_ADDR)" || (echo "Set ERC20_ADDR"; exit 1)
	@test -n "$(ERC_AMOUNT)" || (echo "Set ERC_AMOUNT"; exit 1)
	@AMT=$$(echo "$(ERC_AMOUNT)" | tr -d '_'); \
	cast send "$(HELPER_ADDR)" \
	  'depositErc20(address,uint256,bytes32,bytes32)' \
	  "$(ERC20_ADDR)" \
	  $$AMT \
	  0x1d44d3ef615f749bba2e1e6e777c8660bec189026410cdb055fd1f82b4020000 \
	  0x0000000000000000000000000000000000000000000000000000000000000000 \
	  --rpc-url "$(HYPEREVM_RPC_URL)" \
	  --private-key "$(PRIVATE_KEY)"

# Convenience wrappers for ERC20 deposits
deposit-btc:
	@$(MAKE) cast-deposit-erc20 ERC20_ADDR=$(BTC_ERC20_ADDR) ERC_AMOUNT=$(ERC_AMOUNT)

deposit-usdc:
	@$(MAKE) cast-deposit-erc20 ERC20_ADDR=$(USDC_ERC20_ADDR) ERC_AMOUNT=$(ERC_AMOUNT)

deposit-usdt:
	@$(MAKE) cast-deposit-erc20 ERC20_ADDR=$(USDT_ERC20_ADDR) ERC_AMOUNT=$(ERC_AMOUNT)

# -------------------- Orchestrator ops --------------------

orchestrator-add-liqBtc:
	@echo "AddErc20Arg -> orchestrator"
	dfx deploy ckhype_orchestrator --mode upgrade --upgrade-unchanged --argument '(variant { \
	  AddErc20Arg = record { \
	    contract = record { \
	      chain_id = $(CHAIN_ID_ERC20):nat; \
	      address  = "$(BTC_ERC20_ADDR)"; \
	    }; \
	    ledger_init_arg = record { \
	      decimals        = 8; \
	      transfer_fee    = 10:nat; \
	      token_symbol    = "liqBTC"; \
	      token_name      = "Chain Key Liquidium BTC"; \
	      token_logo      = ""; \
	    }; \
	  } \
	})'

orchestrator-add-liqUSDC:
	@echo "AddErc20Arg -> orchestrator"
	dfx deploy ckhype_orchestrator --mode upgrade --upgrade-unchanged --argument '(variant { \
	  AddErc20Arg = record { \
	    contract = record { \
	      chain_id = $(CHAIN_ID_ERC20):nat; \
	      address  = "$(USDC_ERC20_ADDR)"; \
	    }; \
	    ledger_init_arg = record { \
	      decimals        = 6; \
	      transfer_fee    = 10:nat; \
	      token_symbol    = "liqUSDC"; \
	      token_name      = "Chain Key Liquidium USDC"; \
	      token_logo      = ""; \
	    }; \
	  } \
	})'

orchestrator-add-liqUSDT:
	@echo "AddErc20Arg -> orchestrator (USDT)"
	dfx deploy ckhype_orchestrator --mode upgrade --upgrade-unchanged --argument '(variant { \
	  AddErc20Arg = record { \
	    contract = record { \
	      chain_id = $(CHAIN_ID_ERC20):nat; \
	      address  = "$(USDT_ERC20_ADDR)"; \
	    }; \
	    ledger_init_arg = record { \
	      decimals        = 6; \
	      transfer_fee    = 10_000:nat; \
	      token_symbol    = "liqUSDT"; \
	      token_name      = "Chain Key Liquidium USDT"; \
	      token_logo      = ""; \
	    }; \
	  } \
	})'

orchestrator-upgrade-hashes:
	@if [ -z "$(LEDGER_HASH)" ] || [ -z "$(INDEX_HASH)" ] || [ -z "$(ARCHIVE_HASH)" ]; then \
	  echo "Missing WASM hashes. Run print-hashes."; \
	  exit 1; \
	fi
	dfx deploy ckhype_orchestrator --mode upgrade --upgrade-unchanged --argument '(variant { UpgradeArg = record { \
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

# -------------------- Withdraw helpers --------------------

estimate-eth-fee:
	@MINTER_ID=$$(dfx canister id ckhype_minter); \
	dfx canister call "$$MINTER_ID" eip_1559_transaction_price '(null)'

estimate-erc20-fee:
	@CKERC20_LEDGER="$$CKERC20_LEDGER"; \
	if [ -z "$$CKERC20_LEDGER" ]; then \
	  CKERC20_LEDGER=$$(dfx canister call ckhype_orchestrator canister_ids "(record { chain_id = $(CHAIN_ID_ERC20) : nat; address = \"$(ERC20_ADDR)\" })" | tr -d '\n' | sed -n 's/.*ledger = opt principal \"\([^\"]*\)\".*/\1/p'); \
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

# Thin wrappers using the generic ERC20 helpers
approve-liqBTC-amount:
	@$(MAKE) approve-erc20-amount ERC20_ADDR=$(BTC_ERC20_ADDR)

withdraw-btc:
	@$(MAKE) withdraw-erc20 ERC20_ADDR=$(BTC_ERC20_ADDR)

withdraw-usdc:
	@$(MAKE) withdraw-erc20 ERC20_ADDR=$(USDC_ERC20_ADDR)

withdraw-usdt:
	@$(MAKE) withdraw-erc20 ERC20_ADDR=$(USDT_ERC20_ADDR)

balance:
	@PRINCIPAL=$$(dfx identity get-principal); \
	CKETH_LEDGER=$$(dfx canister id ckhype_ledger); \
	echo "ckETH balance for $$PRINCIPAL:"; \
	dfx canister call "$$CKETH_LEDGER" icrc1_balance_of "(record { owner = principal \"$$PRINCIPAL\"; subaccount = null })"

balance-erc20:
	@$(MAKE) balance-erc20-any ERC20_ADDR=$(ERC20_ADDR)

# -------------------- Introspection --------------------

get-minter-address:
	@test -n "$(HELPER_ADDR)" || (echo "Set HELPER_ADDR"; exit 1)
	@test -n "$(HYPEREVM_RPC_URL)" || (echo "Set HYPEREVM_RPC_URL"; exit 1)
	@echo "Helper reports minter address:"
	cast call "$(HELPER_ADDR)" "getMinterAddress()(address)" --rpc-url "$(HYPEREVM_RPC_URL)"

get-minter-address-ic:
	@MINTER_ID=$$(dfx canister id ckhype_minter); \
	echo "IC minter address:"; \
	dfx canister call "$$MINTER_ID" minter_address

# Deploy CkDeposit helper contract (needs PRIVATE_KEY, HYPEREVM_RPC_URL)
# Uses the IC minter address as constructor arg

deploy-helper:
	@test -n "$(HYPEREVM_RPC_URL)" || (echo "Set HYPEREVM_RPC_URL"; exit 1)
	@test -n "$(PRIVATE_KEY)" || (echo "Set PRIVATE_KEY"; exit 1)
	@MINTER_ID=$$(dfx canister id ckhype_minter); \
	OUT=$$(dfx canister call $$MINTER_ID minter_address | tr -d '\n\r'); \
	MINTER_HEX=$$(echo "$$OUT" | grep -oE '0x[0-9a-fA-F]{40}' | head -n1); \
	test -n "$$MINTER_HEX" || (echo "Failed to obtain minter ETH address from IC (got: $$OUT)"; exit 1); \
	echo "Deploying CkDeposit with minter $$MINTER_HEX"; \
	forge create --broadcast --root ../minter --private-key "$(PRIVATE_KEY)" --rpc-url "$(HYPEREVM_RPC_URL)" \
		DepositHelperWithSubaccount.sol:CkDeposit --constructor-args $$MINTER_HEX