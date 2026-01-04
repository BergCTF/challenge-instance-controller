#!/bin/sh

chall_crd_name="challenges.berg.norelect.ch"
namespace="berg"
instance_name="e2e-current"

challs=$(kubectl get $chall_crd_name -n $namespace -o name)

function create_chall_instance() {
  instance=$1
  name=$2
  cat << EOF | kubectl create -f -
apiVersion: berg.norelect.ch/v1
kind: ChallengeInstance
metadata:
  name: $instance
spec:
  flag: flag{asdf-uwu}
  challengeRef:
    name: $name
    namespace: $namespace
  ownerId: a961d799-7905-484f-b7eb-b4f0fbcf7895
  timeout: "2h"
EOF
}

function wait_for_instance() {
  instance=$1
  name=$2
  # we wait a long time here because we'll likely need to pull container images
  kubectl wait ci/$instance --for=jsonpath='{.status.phase}'=Running --timeout=180s || echo "WARNING, CHALLENGE $name DID NOT COME UP AS HEALTHY"
}

function cleanup_instance() {
  instance=$1

  kubectl delete ci $instance --wait=true
}

function test_chall() {
  ref="$1"
  chall=$(kubectl -n $namespace get $ref -o json)

  name=$(echo $chall | jq -r '.metadata.name')

  containers=$(echo $chall | jq '.spec.containers | length')
  if [ $containers -eq 0 ]; then
    echo "skipping $name, no containers"
    return
  fi

  echo "testing $name"
  create_chall_instance $instance_name $name

  echo "waiting for challenge instance"
  wait_for_instance $instance_name $name

  cleanup_instance $instance_name
}

for ref in $challs; do
  test_chall $ref
done
