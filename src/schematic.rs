use k8s_openapi::api::core::v1 as core;
use k8s_openapi::apimachinery::pkg::{
    api::resource::Quantity,
    util::intstr::IntOrString,
};

use std::collections::BTreeMap;

/// The default workload type if none is present.
pub const DEFAULT_WORKLOAD_TYPE: &str = "core.hydra.io/v1alpha1.Singleton";

/// Component describes the "spec" of a Hydra component schematic.
/// 
/// The wrapper of the schematic is provided by the Kubernetes library natively.
/// 
/// In addition to directly deserializing into a component, the from_string() helper
/// can be used for testing and prototyping.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct Component {
    pub workload_type: String,
    pub os_type: String,
    pub arch: String,
    pub parameters: Vec<Parameter>,
    pub containers: Vec<Container>,
    pub workload_settings: Vec<WorkloadSetting>,
}

impl Component {
    /// Parse JSON data into a Component.
    pub fn from_str(json_data: &str) -> Result<Component, failure::Error> {
        let res: Component = serde_json::from_str(json_data)?;
        Ok(res)
    }

    /// to_pod_spec generates a pod specification.
    pub fn to_pod_spec(&self) -> core::PodSpec {
        let containers = self.containers.iter().map(|c| {
            core::Container {
                name: c.name.clone(),
                image:     Some(c.image.clone()),
                resources: Some(c.resources.to_resource_requirements()),
                ports:     Some(c.ports.iter().map(|p| { p.to_container_port() }).collect()),
                env:       Some(c.env.iter().map(|e| { e.to_env_var() }).collect()),
                liveness_probe:  c.liveness_probe.clone().and_then( |p| Some(p.to_probe())),
                readiness_probe: c.readiness_probe.clone().and_then(|p| Some(p.to_probe())),
                ..Default::default()
            }
        }).collect();
        core::PodSpec {
            containers: containers,
            ..Default::default()
        }
    }
}

impl Into<core::PodSpec> for Component {
    fn into(self) -> core::PodSpec {
        self.to_pod_spec()
    }
}



impl Default for Component {
    fn default() -> Self {
        Component{
            workload_type: DEFAULT_WORKLOAD_TYPE.into(),
            os_type: "linux".into(),
            arch: "amd64".into(),
            parameters: Vec::new(),
            containers: Vec::new(),
            workload_settings: Vec::new(),
        }
    }
}

/// Application defines a Hydra application
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Application {

}

/// Trait describes Hydra traits.
/// 
/// Hydra traits are ops-oriented "add-ons" that can be attached to Components of the appropriate workloadType.
/// For example, an autoscaler trait can attach to a workloadType (such as ReplicableService) that can be
/// scaled up and down.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Trait {

}

/// Parameter describes a configurable unit on a Component or Application.
/// 
/// Parameters have primitive types, and may be marked as required. Default values
/// may be provided as well.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Parameter {
    pub name: String,
    pub description: Option<String>,

    #[serde(rename(serialize = "type", deserialize = "type"))]
    pub parameter_type: ParameterType,

    #[serde(default = "default_required")]
    pub required: bool,

    pub default: Option<serde_json::Value>,
}

/// Supplies the default value for all required fields.
fn default_required() -> bool {
    false
}

/// Container describes the container configuration for a Component.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Container {
    pub name: String,
    pub image: String,

    #[serde(default)]
    pub resources: Resources,
    
    #[serde(default)]
    pub env: Vec<Env>,
    
    #[serde(default)]
    pub ports: Vec<Port>,
    
    pub liveness_probe: Option<HealthProbe>,
    pub readiness_probe: Option<HealthProbe>,
}

/// Workload settings describe the configuration for a workload.
/// 
/// This information is passed to the underlying workload defined by Component::worload_type.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct WorkloadSetting{
    pub name: String,
    pub description: Option<String>,

    #[serde(rename(serialize = "type", deserialize = "type"))]
    pub parameter_type: ParameterType,

    #[serde(default = "default_required")]
    pub required: bool,
    
    pub default: Option<serde_json::Value>,
    pub from_param: Option<String>,
}

/// Env describes an environment variable for a container.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Env{
    pub name: String,
    pub value: Option<String>,
    pub from_param: Option<String>,
}
impl Env {
    fn to_env_var(&self) -> core::EnvVar {
        // FIXME: This needs to support fromParam
        core::EnvVar {
            name: self.name.clone(),
            value: self.value.clone(),
            value_from: None,
        }
    }
}

/// Port describes a port on a Container.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Port{
    pub name: String,
    pub container_port: i32,

    #[serde(default)]
    pub protocol: PortProtocol,
}
impl Port {
    fn to_container_port(&self) -> core::ContainerPort {
        core::ContainerPort {
            container_port: self.container_port,
            name: Some(self.name.clone()),
            protocol: Some(self.protocol.to_string()),
            ..Default::default()
        }
    }
}

// HealthProbe describes a probe used to check on the health of a Container.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct HealthProbe{
    pub exec: Option<Exec>,
    pub http_get: Option<HttpGet>,
    pub tcp_socket: Option<TcpSocket>,
    pub initial_delay_seconds: i32,
    pub period_seconds: i32,
    pub timeout_seconds: i32,
    pub success_threshold: i32,
    pub failure_threshold: i32,
}
impl HealthProbe {
    fn to_probe(&self) -> core::Probe {
        core::Probe {
            failure_threshold:     Some(self.failure_threshold),
            period_seconds:        Some(self.period_seconds),
            timeout_seconds:       Some(self.timeout_seconds),
            success_threshold:     Some(self.success_threshold),
            initial_delay_seconds: Some(self.initial_delay_seconds),
            exec:       self.exec.clone().and_then(      |c| { Some(core::ExecAction{command: Some(c.command)}) }),
            http_get:   self.http_get.clone().and_then(  |a| { Some(a.to_http_get_action()) }),
            tcp_socket: self.tcp_socket.clone().and_then(|t| { Some(t.to_tcp_socket_action()) }),
        }
    }
}
impl Default for HealthProbe {
    fn default() -> Self {
        HealthProbe{
            exec: None,
            http_get: None,
            tcp_socket: None,
            initial_delay_seconds: 0,
            period_seconds: 10,
            timeout_seconds: 1,
            success_threshold: 1,
            failure_threshold: 3,
        }
    }
}

/// Exec describes a shell command, as an array, for execution in a Container.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Exec {
    pub command: Vec<String>,
}

/// HttpGet describes an HTTP GET request used to probe a container.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct HttpGet{
    pub path: String,
    pub port: i32,
    pub http_headers: Vec<HttpHeader>,
}
impl HttpGet {
    fn to_http_get_action(&self) -> core::HTTPGetAction {
        core::HTTPGetAction {
            http_headers: Some(self.http_headers.iter().map(|h|{h.to_kube_header()}).collect()),
            path: Some(self.path.clone()),
            port: IntOrString::Int(self.port),
            ..Default::default()
        }
    }
}

/// HttpHeader describes an HTTP header.
///
/// Headers are not stored as a map of name/value because the same header is allowed
/// multiple times.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct HttpHeader{
    pub name:  String,
    pub value: String,
}
impl HttpHeader {
    fn to_kube_header(&self) -> core::HTTPHeader {
        core::HTTPHeader{
            name:  self.name.clone(),
            value: self.value.clone(),
        }
    }
}

/// TcpSocket defines a socket used for health probing.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TcpSocket{
    pub port: i32,
}
impl TcpSocket {
    fn to_tcp_socket_action(&self) -> core::TCPSocketAction {
        core::TCPSocketAction {
            port: IntOrString::Int(self.port),
            ..Default::default()
        }
    }
}

/// Resources defines the resources required by a container.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct Resources{
    pub cpu: CPU,
    pub memory: Memory,
    pub gpu: GPU,
    pub paths: Vec<Path>,
}
impl Resources {
    fn to_resource_requirements(&self) -> core::ResourceRequirements {
            let mut requests = BTreeMap::new();
            requests.insert("cpu".to_string(), Quantity(self.cpu.required.clone()));
            requests.insert("memory".to_string(), Quantity(self.memory.required.clone()));

            // TODO: Kubernetes does not have a built-in type for GPUs. What do we use?
            core::ResourceRequirements{
                requests: Some(requests),
                limits: None,
            }
    }
}

impl Default for Resources {
    fn default() -> Self {
        Resources {
            cpu: CPU{required: "1".into()},
            memory: Memory{required: "1G".into()},
            gpu: GPU{required: "0".into()},
            paths: Vec::new(),
        }
    }
}

/// CPU describes a CPU resource allocation for a container.
/// 
/// It indicates how much CPU (core count) is required for this container to operate.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CPU{
    pub required: String,
}

/// Memory describes the memory allocation for a container.
/// 
/// It indicates the required amount of memory for a container to operate.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Memory{
    pub required: String,
}

/// GPU describes a Container's need for a GPU.
/// 
/// It indicates how many (if any) GPU cores a container needs to operate.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GPU{
    pub required: String,
}

/// Path describes a path that is attached to a Container.
/// 
/// It specifies not only the location, but also the requirements.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Path{
    pub name: String,
    pub path: String,

    #[serde(default)]
    pub access_mode: AccessMode,
    
    #[serde(default)]
    pub sharing_policy: SharingPolicy,
}

/// ParameterType defines the types of parameters for a Parameters object.
///
/// These roughly correlate with JSON Schema primitive types.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum ParameterType {
    Boolean,
    String,
    Number,
    Null,
}

/// AccessMode defines the access modes for file systems.
/// 
/// Currently, only read/write and read-only are supported.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum AccessMode{
    RW,
    RO,
}
impl Default for AccessMode {
    fn default() -> Self {
        AccessMode::RW
    }
}

/// SharingPolicy defines whether a filesystem can be shared across containers.
/// 
/// An Exclusive filesystem can only be attached to one container.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum SharingPolicy{
    Shared,
    Exclusive,
}
impl Default for SharingPolicy {
    fn default() -> Self {
        SharingPolicy::Exclusive
    }
}

/// PortProtocol is a protocol used when attaching to ports.
/// 
/// Currently, only TCP and UDP are supported by Kubernetes.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum PortProtocol{
    TCP,
    UDP,
    SCTP,
}
impl PortProtocol {
    fn as_str(&self) -> &str {
        match self {
            PortProtocol::UDP => "UDP",
            PortProtocol::SCTP => "SCTP",
            PortProtocol::TCP => "TCP",
        }
    }
}
impl Default for PortProtocol {
    fn default() -> Self {
        PortProtocol::TCP
    }
}
impl ToString for PortProtocol {
    fn to_string(&self) -> String {
        self.as_str().to_string()
    }
    
}

// TODO: This part is not specified in the spec b/c it is considered a runtime
// detail of Kubernetes. Need to fill this in as we go.

/// HydraStatus is the status of a Hydra object, per Kubernetes.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct HydraStatus {
    pub phase: Option<String>,
}
impl Default for HydraStatus {
    fn default() -> Self {
        HydraStatus {
            phase: None,
        }
    }
}

/// Status is a convenience for an optional HydraStatus.
pub type Status = Option<HydraStatus>;

/// GroupVersionKind represents a fully qualified identifier for a resource type.
/// 
/// It is, as the name suggests, composed of three pieces of information:
/// - Group is a namespace
/// - Version is an API version
/// - Kind is the actual type marker
pub struct GroupVersionKind {
    pub group: String,
    pub version: String,
    pub kind: String,
}

/// GroupVersionKind represents a canonical name, composed of group, version, and (you guessed it) kind.
/// 
/// Group is a dotted name. While the specification requires at least one dot in the group, we do not enforce.
/// Version is an API version
/// Kind the name of the type
impl GroupVersionKind {
    /// Create a new GroupVersionKind from each component.
    /// 
    /// This does not check the formatting of each part.
    pub fn new(group: &str, version: &str, kind: &str) -> GroupVersionKind {
        GroupVersionKind{
            group: group.into(),
            version: version.into(),
            kind: kind.into(),
        }
    }
    /// Parse a string into a GroupVersionKind.
    pub fn from_str(gvp: &str) -> Result<GroupVersionKind, failure::Error> {
        // I suspect that this function could be made much more elegant.
        let parts: Vec<&str> = gvp.splitn(2, "/").collect();
        if parts.len() != 2 {
            return Err(failure::err_msg("missing version and kind"))
        }

        let vk: Vec<&str> = parts.get(1).unwrap().splitn(2, ".").collect();
        if vk.len() != 2 {
            return Err(failure::err_msg("missing kind"))
        }

        Ok(GroupVersionKind{
            group: parts.get(0).unwrap().to_string(),
            version: vk.get(0).unwrap().to_string(),
            kind: vk.get(1).unwrap().to_string(),
        })
    }
}