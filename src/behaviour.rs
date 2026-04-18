use libp2p::swarm::dummy;

pub type OrinoxBehaviour = dummy::Behaviour;

pub fn create_behaviour() -> OrinoxBehaviour {
    dummy::Behaviour
}

