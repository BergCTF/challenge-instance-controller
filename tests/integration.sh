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

docker build -t berg-controller:test -f Dockerfile .
./tests/integration/setup-kind.sh || ./tests/integration/teardown-kind.sh
./tests/integration/run-tests.sh || collect_logs
./tests/integration/teardown-kind.sh
