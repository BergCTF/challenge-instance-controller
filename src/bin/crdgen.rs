use kube::CustomResourceExt;

fn main() {
    let crds = vec![
        berg_operator::crd::Challenge::crd(),
        berg_operator::crd::ChallengeInstance::crd(),
    ];
    print!("{}", serde_yaml::to_string(&crds).unwrap());
}
