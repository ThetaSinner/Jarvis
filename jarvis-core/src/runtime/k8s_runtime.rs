use crate::runtime::BuildRuntime;

pub struct KubernetesRuntime {

}

impl BuildRuntime for KubernetesRuntime {
    fn test(&self) {
        println!("I'm the kubernetes runtime");
    }
}
