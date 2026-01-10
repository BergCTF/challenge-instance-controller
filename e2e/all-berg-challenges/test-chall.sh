
namespace="berg"
chall_crd_name="challenges.berg.norelect.ch"
instance_prefix="e2e-current"

function create_chall_instance() {
  instance=$1
  name=$2
  cat << EOF | kubectl create -f - 2>&1 > /dev/null
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
  kubectl wait ci/$instance --for=jsonpath='{.status.phase}'=Running --timeout=180s 2>&1 > /dev/null || echo "WARNING, CHALLENGE $name DID NOT COME UP AS HEALTHY"
}

function cleanup_instance() {
  instance=$1

  kubectl delete ci $instance --wait=true 2>&1 > /dev/null
}

ref="$1"
chall=$(kubectl -n $namespace get $ref -o json)

name=$(echo $chall | jq -r '.metadata.name')

containers=$(echo $chall | jq '.spec.containers | length')
if [ $containers -eq 0 ]; then
  exit 0
fi

instance_name="$instance_prefix-$name"

create_chall_instance $instance_name $name
wait_for_instance $instance_name $name
cleanup_instance $instance_name
