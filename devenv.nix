{
  pkgs,
  lib,
  config,
  inputs,
  ...
}:
{
  # https://devenv.sh/basics/
  env.GREET = "devenv";

  # https://devenv.sh/packages/
  packages = [
    pkgs.git
    pkgs.kubernetes-helm
    pkgs.kubectl
    pkgs.kind
  ];

  # https://devenv.sh/languages/
  languages.rust.enable = true;

  # https://devenv.sh/processes/
  # processes.dev.exec = "${lib.getExe pkgs.watchexec} -n -- ls -la";

  # https://devenv.sh/services/
  # services.postgres.enable = true;

  # https://devenv.sh/scripts/
  # scripts.hello.exec = ''
  #   echo hello from $GREET
  # '';

  # https://devenv.sh/basics/
  # enterShell = ''
  #   hello         # Run scripts directly
  # '';

  # https://devenv.sh/tasks/
  # tasks = {
  #   "myproj:setup".exec = "mytool build";
  #   "devenv:enterShell".after = [ "myproj:setup" ];
  # };

  tasks."tests:integration" = {
    exec = ''
      function collect_logs() {
        echo "==> Operator logs:"
        kubectl logs -l app.kubernetes.io/name=berg-operator -n berg-test --tail=200 || true
        echo ""
        echo "==> ChallengeInstance status:"
        kubectl get challengeinstance -n berg-test -o yaml || true
        echo ""
        echo "==> Events:"
        kubectl get events -n berg-test --sort-by='.lastTimestamp' || true
      }

      devenv tasks run build:release
      ./tests/integration/setup-kind.sh || ./tests/integration/teardown-kind.sh
      ./tests/integration/run-tests.sh || collect_logs
      ./tests/integration/teardown-kind.sh
    '';
  };

  tasks."build:release" = {
    exec = ''
      docker build -t berg-controller:test -f Dockerfile .
    '';
  };

  # https://devenv.sh/tests/
  enterTest = ''
    cargo test
  '';

  # https://devenv.sh/git-hooks/
  # git-hooks.hooks.shellcheck.enable = true;
  git-hooks.hooks = {
    # github actions
    actionlint.enable = true;
    action-validator.enable = true;

    # might as well lint nix
    nil.enable = true;
    nixfmt-rfc-style.enable = true;

    # rust
    clippy.enable = true;
    clippy.settings.allFeatures = true;
    clippy.settings.denyWarnings = true;
    cargo-check.enable = true;
    rustfmt.enable = true;

    # helm
    # this one just runs ct lint --all --skip-dependencies
    chart-testing.enable = true;
  };

  # See full reference at https://devenv.sh/reference/options/
}
