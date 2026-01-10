#!/bin/sh

chall_crd_name="challenges.berg.norelect.ch"
namespace="berg"
instance_prefix="e2e-current"

challs=$(kubectl get $chall_crd_name -n $namespace -o name)

echo $challs | xargs -n 1 -P 3 sh e2e/all-berg-challenges/test-chall.sh
