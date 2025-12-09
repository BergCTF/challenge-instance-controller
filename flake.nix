{
  description = "Berg Challenge Instance Controller";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" ];
        };

      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            # Rust toolchain
            rustToolchain

            # Kubernetes tools
            kubectl
            kind
            kubernetes-helm

            # Container tools
            docker

            # Testing tools
            jq
            yq-go

            # Development tools
            git
            openssl
            pkg-config
          ];

          shellHook = ''
            echo "Berg Operator Development Environment"
            echo "======================================"
            echo "Rust:       $(rustc --version)"
            echo "Cargo:      $(cargo --version)"
            echo "kubectl:    $(kubectl version --client --short 2>/dev/null || echo 'not in cluster')"
            echo "kind:       $(kind version)"
            echo "helm:       $(helm version --short)"
            echo ""
            echo "Integration Tests:"
            echo "  ./tests/integration/setup-kind.sh   - Set up kind cluster"
            echo "  ./tests/integration/run-tests.sh    - Run integration tests"
            echo "  ./tests/integration/teardown-kind.sh - Clean up cluster"
          '';
        };
      }
    );
}
