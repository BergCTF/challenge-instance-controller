.PHONY: help build test crdgen docker-build clean install-crds deploy

# Default target
help:
	@echo "Berg Operator - Development Makefile"
	@echo ""
	@echo "Available targets:"
	@echo "  build         - Build the operator binary"
	@echo "  test          - Run unit tests"
	@echo "  crdgen        - Generate CRD YAML files"
	@echo "  docker-build  - Build Docker image"
	@echo "  install-crds  - Install CRDs to current kubectl context"
	@echo "  deploy        - Deploy operator via Helm"
	@echo "  clean         - Clean build artifacts"

# Build the operator
build:
	cargo build --release

# Run tests
test:
	cargo test

# Generate CRD YAML files
crdgen:
	@echo "Generating CRDs..."
	cargo run --bin crdgen 2>/dev/null > charts/berg-operator/templates/crd.yaml
	@echo "CRDs generated to charts/berg-operator/templates/crd.yaml"

# Build Docker image
docker-build:
	docker build -t berg-operator:latest .

# Install CRDs to current cluster
install-crds:
	kubectl apply -f charts/berg-operator/templates/crd.yaml

# Deploy operator via Helm (with default values)
deploy:
	helm upgrade --install berg-operator charts/berg-operator \
		--create-namespace \
		--namespace berg-system

# Clean build artifacts
clean:
	cargo clean
	rm -f charts/berg-operator/templates/crd.yaml

# Development shortcuts
dev-build: build test crdgen
	@echo "Development build complete!"

# Integration test setup
test-setup:
	./tests/integration/setup-kind.sh

# Integration test run
test-integration:
	./tests/integration/run-tests.sh

# Integration test teardown
test-teardown:
	./tests/integration/teardown-kind.sh
