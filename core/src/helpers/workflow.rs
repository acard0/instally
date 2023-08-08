
#[derive(Clone, Debug)]
pub enum Workflow {
    FreshInstallition,
    MaintinanceTool,
    FfiApi,
}

pub fn get_workflow() -> Workflow {
    if std::env::var("STANDALONE_EXECUTION").is_ok() {
        return Workflow::FreshInstallition;
    }

    if std::env::var("MAINTINANCE_EXECUTION").is_ok() {
        return Workflow::MaintinanceTool;
    }

    return Workflow::FfiApi;
}