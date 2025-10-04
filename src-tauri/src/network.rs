use local_ip_address::local_ip;

/// Attempts to discover the local, non-loopback IP address of the machine.
///
/// # Returns
/// An `Option<String>` containing the IP address if found, otherwise `None`.
pub fn get_local_ip_address() -> Option<String> {
    local_ip().ok().map(|ip| ip.to_string())
}
