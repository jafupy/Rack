#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Destination {
    Loopback { port: u16 },
}

impl Destination {
    pub fn loopback(port: u16) -> Self {
        Self::Loopback { port }
    }

    pub(crate) fn port(&self) -> u16 {
        match self {
            Self::Loopback { port } => *port,
        }
    }

    pub fn loopback_url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceTarget {
    pub service_id: String,
    pub host: String,
    pub port: u16,
}

impl ServiceTarget {
    pub fn destination(&self) -> Destination {
        Destination::loopback(self.port)
    }

    pub fn loopback_url(&self) -> String {
        self.destination().loopback_url()
    }
}
