use rmcp::{
    Error as McpError, ServerHandler,
    handler::server::tool::{Parameters, ToolRouter},
    model::*,
    schemars, tool, tool_handler, tool_router,
};

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

#[tool_router]
impl UnitConversion {
    #[tool(
        description = "Convert from one unit to another. Provide the original value and the desired output unit"
    )]
    async fn convert_units(
        &self,
        Parameters(ConversionRequest {
            input_value,
            output_unit,
        }): Parameters<ConversionRequest>,
    ) -> Result<CallToolResult, McpError> {
        let result = crate::convert_units(&input_value, &output_unit)
            .map_err(|e| McpError::new(ErrorCode::INVALID_REQUEST, e.to_string(), None))?;

        Ok(CallToolResult::success(vec![Content::text(result)]))
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
