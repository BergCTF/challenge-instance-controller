{
  description = "berg-challenge-instance-controller";

  inputs = {
    systems.url = "github:nix-systems/default";
    git-hooks.url = "github:cachix/git-hooks.nix";
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    nixpkgs-stable.url = "github:nixos/nixpkgs/nixos-25.11";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      systems,
      nixpkgs,
      nixpkgs-stable,
      rust-overlay,
      ...
    }@inputs:
    let
      overlays = [ (import rust-overlay) ];
      forEachSystem = nixpkgs.lib.genAttrs (import systems);
    in
    {
      # Run the hooks with `nix fmt`.
      formatter = forEachSystem (
        system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
          config = self.checks.${system}.pre-commit-check.config;
          inherit (config) package configFile;
          script = ''
            ${pkgs.lib.getExe package} run --all-files --config ${configFile}
          '';
        in
        pkgs.writeShellScriptBin "pre-commit-run" script
      );

      # Run the hooks in a sandbox with `nix flake check`.
      # Read-only filesystem and no internet access.
      checks = forEachSystem (system: {
        pre-commit-check = inputs.git-hooks.lib.${system}.run {
          src = ./.;
          hooks = {
            # github actions
            actionlint.enable = true;
            action-validator.enable = true;

            # might as well lint nix
            nil.enable = true;
            nixfmt-rfc-style.enable = true;

            # rust
            # clippy requires internet so it doesn't work in a check
            # clippy.enable = true;
            # clippy.settings.allFeatures = true;
            # clippy.settings.denyWarnings = true;
            # cargo-check.enable = true;
            rustfmt.enable = true;

            # helm
            # this one just runs ct lint --all --skip-dependencies
            # chart-testing.enable = true;
          };
        };
      });

      # Enter a development shell with `nix develop`.
      # The hooks will be installed automatically.
      # Or run pre-commit manually with `nix develop -c pre-commit run --all-files`
      devShells = forEachSystem (system: {
        default =
          let
            pkgs = import nixpkgs {
              inherit system overlays;
            };
            pkgs-stable = import nixpkgs-stable {
              inherit system overlays;
            };
            inherit (self.checks.${system}.pre-commit-check) shellHook enabledPackages;
          in
          pkgs.mkShell {
            inherit shellHook;
            packages = [
              pkgs.git
              pkgs.kubernetes-helm
              pkgs.kubectl
              pkgs.kind
              pkgs-stable.kyverno-chainsaw

              pkgs.cargo-outdated
              pkgs.cargo-machete
              pkgs.cargo-edit
              pkgs.cargo-insta
              pkgs.cargo-deny

              pkgs.cargo
              pkgs.rustc
              pkgs.rust-bin.beta.latest.default
            ]
            ++ enabledPackages;
          };
      });
    };
}
