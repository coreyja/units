# Units MCP Server

A simple Model Context Protocol (MCP) server that provides unit conversion capabilities. This server demonstrates how to build an MCP-compatible service that works with Claude and other MCP-supporting clients.

## Features

- Convert between common measurement units
- Support for multiple unit types including length, mass, temperature, volume, velocity, area, density, acceleration, force, energy, power, and fuel economy
- Simple REST API with SSE transport
- Compatible with any MCP-supporting client

## Installation

### Prerequisites

- Rust 1.70 or higher
- PostgreSQL database (for production deployment)
- Git

### Local Development

1. Clone the repository:
```bash
git clone https://github.com/coreyja/units.git
cd units
```

2. Install dependencies:
```bash
cargo build
```

3. Set up environment variables:
```bash
export DATABASE_URL="postgresql://user:password@localhost/units_db"
export PORT=3001  # Optional, defaults to 3001
```

4. Run the development server:
```bash
cargo run --bin server
```

The server will be available at `http://localhost:3001`

### Production Deployment

This project is configured for deployment on Fly.io:

```bash
fly deploy
```

## Usage Examples

### Using with Claude or other MCP Clients

1. Add the MCP server URL to your client configuration:
   - URL: `https://units.coreyja.com/mcp/sse`
   - Transport: SSE (Server-Sent Events)

2. Once connected, you can use the `convert_units` tool in your conversations:

```
User: Convert 100 meters to feet
Assistant: I'll convert 100 meters to feet for you.
[Uses convert_units tool]
Result: 100 meters equals 328.084 feet
```

### Direct API Usage

The server exposes an MCP-compatible API that can be accessed directly:

#### Convert Units Tool

The main tool available is `convert_units` which takes two parameters:
- `input_value`: The value to convert (e.g., "10 meters", "32 fahrenheit")
- `output_unit`: The desired output unit (e.g., "feet", "celsius")

Example conversation flow:
```
1. Connect to the SSE endpoint at /mcp/sse
2. Send a tool call request for convert_units
3. Receive the conversion result
```

### Supported Unit Types

#### Length
- meters, feet, kilometers, miles

#### Mass
- kilograms, pounds, grams

#### Temperature
- celsius, fahrenheit

#### Volume
- liters, gallons, milliliters
- cubic meters, cubic feet, cubic inches

#### Velocity
- mph, km/h, m/s, ft/s

#### Area
- square meters/feet/kilometers/miles, acres

#### Density
- kg/m³, lb/ft³, g/cm³, g/mL

#### Acceleration
- m/s², ft/s²

#### Force
- newtons, pounds force

#### Energy
- joules, foot pounds

#### Power
- watts, horsepower

#### Fuel Economy
- miles/gallon, km/L, L/100km

## API Documentation

### Endpoints

#### `GET /`
Returns the homepage with documentation and examples

#### `GET /mcp/sse`
SSE endpoint for MCP communication

#### `POST /mcp/message`
Message endpoint for MCP requests

### MCP Tool: convert_units

**Description**: Convert from one unit to another

**Parameters**:
- `input_value` (string, required): The input value with unit (e.g., "10 meters")
- `output_unit` (string, required): The desired output unit (e.g., "feet")

**Example Request**:
```json
{
  "tool": "convert_units",
  "parameters": {
    "input_value": "100 kilometers",
    "output_unit": "miles"
  }
}
```

**Example Response**:
```json
{
  "result": "62.1371 miles"
}
```

## Configuration

### Environment Variables

- `DATABASE_URL` (required): PostgreSQL connection string
- `PORT` (optional): Server port, defaults to 3001
- `SENTRY_DSN` (optional): Sentry error tracking DSN

### Database Setup

The server uses PostgreSQL with automatic migrations. Ensure your database user has appropriate permissions for schema migrations.

## Contributing Guidelines

We welcome contributions to the Units MCP server! Here's how you can help:

### Development Setup

1. Fork the repository
2. Create a feature branch: `git checkout -b feature/your-feature-name`
3. Make your changes following the coding standards
4. Write tests for new functionality
5. Ensure all tests pass: `cargo test`
6. Commit your changes with descriptive messages
7. Push to your fork and submit a pull request

### Code Style

- Follow Rust standard formatting: `cargo fmt`
- Ensure no clippy warnings: `cargo clippy`
- Write descriptive variable and function names
- Add comments for complex logic
- Keep functions focused and small

### Testing

- Write unit tests for new conversion functions
- Test edge cases (zero, negative values, very large numbers)
- Verify error handling for invalid inputs

### Adding New Units

To add support for new units:

1. Update the `get_unit_type()` function in `src/unit_conversion.rs`
2. Add the conversion logic in the appropriate conversion function
3. Add unit tests for the new conversions
4. Update the README documentation

### Reporting Issues

- Use the GitHub issue tracker
- Include steps to reproduce the problem
- Provide example inputs that cause the issue
- Include error messages if applicable

### Pull Request Process

1. Update the README.md with details of changes if applicable
2. Add tests for new functionality
3. Ensure CI passes all checks
4. Request review from maintainers
5. Address review feedback promptly

### Code of Conduct

- Be respectful and inclusive
- Welcome newcomers and help them get started
- Focus on constructive feedback
- Maintain a positive environment

## License

This project is open source. Please check the repository for license details.

## Support

For issues, questions, or contributions:
- GitHub Issues: https://github.com/coreyja/units/issues
- Author: [@coreyja](https://coreyja.com)