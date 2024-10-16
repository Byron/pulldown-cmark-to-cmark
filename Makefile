
help:  ## Display this help
	@awk 'BEGIN {FS = ":.*##"; printf "\nUsage:\n  make \033[36m<target>\033[0m\n"} /^[a-zA-Z_-]+:.*?##/ { printf "  \033[36m%-15s\033[0m %s\n", $$1, $$2 } /^##@/ { printf "\n\033[1m%s\033[0m\n", substr($$0, 5) } ' $(MAKEFILE_LIST)

.PHONY: docs unittests journeytests tests

##@ Testing

docs: ## Builds docs
	cargo doc

unittests: ## run unit tests
	cargo test --all

clippy: ## run clippy
	cargo clippy

journeytests: ## run journey tests
	./tests/cat.sh

fuzz: ## run fuzz tests for 30 seconds
	cargo install cargo-fuzz
	cargo fuzz run round-trip -- -only_ascii=1 -max_total_time=30

tests: docs clippy unittests journeytests fuzz  ## run all tests
