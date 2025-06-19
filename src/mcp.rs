use rmcp::{Error as McpError, model::*, tool};

#[derive(Clone)]
pub struct UnitConversion {}

#[tool(tool_box)]
impl UnitConversion {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {}
    }

    #[tool(
        description = "Convert from one unit to another. Provide the original value and the desired output unit"
    )]
    async fn convert_units(
        &self,
        #[tool(param)]
        #[schemars(description = "Input")]
        input_value: String,
        #[tool(param)]
        #[schemars(description = "Output Unit")]
        output_unit: String,
    ) -> Result<CallToolResult, McpError> {
        let result = crate::convert_units(&input_value, &output_unit)
            .map_err(|e| McpError::new(ErrorCode::INVALID_REQUEST, e.to_string(), None))?;

        Ok(CallToolResult::success(vec![Content::text(result)]))
    }
}
