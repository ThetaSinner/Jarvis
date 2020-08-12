use crate::runtime::BuildRuntime;

pub struct DockerRuntime {

}

impl BuildRuntime for DockerRuntime {
    fn test(&self) {
        println!("I'm the docker runtime");
    }
}
