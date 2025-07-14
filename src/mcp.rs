use rmcp::{
    Error as McpError, ServerHandler,
    handler::server::tool::{Parameters, ToolRouter},
    model::*,
    schemars, tool, tool_handler, tool_router,
};
use tracing::{info, error, instrument};

#[derive(Clone)]
pub struct UnitConversion {
    pub tool_router: ToolRouter<UnitConversion>,
}

impl UnitConversion {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }
}

impl Default for UnitConversion {
    fn default() -> Self {
        Self::new()
    }
}

#[tool_router(vis = "pub")]
impl UnitConversion {
    #[tool(
        description = "Convert from one unit to another. Provide the original value and the desired output unit"
    )]
    #[instrument(skip(self), fields(input = %input_value, output_unit = %output_unit))]
    async fn convert_units(
        &self,
        Parameters(ConversionRequest {
            input_value,
            output_unit,
        }): Parameters<ConversionRequest>,
    ) -> Result<CallToolResult, McpError> {
        info!("Received conversion request");
        
        match crate::convert_units(&input_value, &output_unit) {
            Ok(result) => {
                info!(
                    input = %input_value,
                    output_unit = %output_unit,
                    result = %result,
                    "Conversion successful"
                );
                Ok(CallToolResult::success(vec![Content::text(result)]))
            }
            Err(e) => {
                error!(
                    input = %input_value,
                    output_unit = %output_unit,
                    error = %e,
                    "Conversion failed"
                );
                
                // Provide user-friendly error messages
                let user_message = match &e {
                    crate::ConversionError::InvalidInputFormat => {
                        "Invalid input format. Please provide a value followed by a unit (e.g., '10 meters')".to_string()
                    }
                    crate::ConversionError::UnknownUnit(unit) => {
                        format!("Unknown unit '{unit}'. Please check the spelling and try again.")
                    }
                    crate::ConversionError::IncompatibleUnits { from, to } => {
                        format!("Cannot convert between {from} and {to} - they are different types of measurements.")
                    }
                    _ => e.to_string(),
                };
                
                Err(McpError::new(ErrorCode::INVALID_REQUEST, user_message, None))
            }
        }
    }
}

#[tool_handler]
impl ServerHandler for UnitConversion {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            instructions: Some(
                "A unit conversion server that can convert between various units of measurement"
                    .into(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation {
                name: "Units".into(),
                version: "0.1.0".into(),
            },
        }
    }
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ConversionRequest {
    #[schemars(description = "the input value")]
    pub input_value: String,
    #[schemars(description = "the output unit")]
    pub output_unit: String,
}
