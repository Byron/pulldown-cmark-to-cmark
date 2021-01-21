
help:  ## Display this help
	@awk 'BEGIN {FS = ":.*##"; printf "\nUsage:\n  make \033[36m<target>\033[0m\n"} /^[a-zA-Z_-]+:.*?##/ { printf "  \033[36m%-15s\033[0m %s\n", $$1, $$2 } /^##@/ { printf "\n\033[1m%s\033[0m\n", substr($$0, 5) } ' $(MAKEFILE_LIST)

.PHONY: docs unittests journeytests tests

##@ Testing

docs: ## Builds docs
	cargo doc

unittests: ## run unit tests
	cargo test --all

journeytests: ## run journey tests
	./tests/cat.sh


tests: docs unittests journeytests  ## run all tests
