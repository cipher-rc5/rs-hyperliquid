This Rust codebase implements a modular, event-driven WebSocket client for Hyperliquid's trading API. Here's the architectural breakdown:

## Core Architecture Pattern

The system follows an **Event-Driven Architecture** with clear separation of concerns:

```
┌─────────────────┐    Events    ┌─────────────────┐
│   Client Layer  │ ──────────►  │   UI Layer      │
│  (WebSocket)    │              │ (Presentation)  │
└─────────────────┘              └─────────────────┘
        │                                │
        ▼                                ▼
┌─────────────────┐              ┌─────────────────┐
│  State Manager  │              │   Formatters    │
│   (Shared)      │              │   (Output)      │
└─────────────────┘              └─────────────────┘
```

## Key Architectural Components

### 1. **Event System** (`events.rs`)
- **Purpose**: Decouples client logic from UI presentation
- **Pattern**: Publisher-Subscriber with typed events
- **Key Types**: `ClientEvent` enum with variants like `TradeReceived`, `Connected`, `Reconnecting`
- **Implementation**: Uses Tokio's unbounded channels for async communication

### 2. **State Management** (`client_state.rs`)
- **Purpose**: Thread-safe state sharing between components
- **Pattern**: Shared mutable state with Arc<Mutex<>>
- **Responsibilities**: Connection tracking, metrics, reconnection counting
- **Thread Safety**: Uses atomic operations for counters, mutex for complex state

### 3. **Client Layer** (`client.rs`)
- **Purpose**: WebSocket connection management and message handling
- **Pattern**: Actor-like behavior with message processing loop
- **Key Features**:
  - Automatic reconnection with exponential backoff
  - Message parsing and routing
  - Health monitoring
  - Subscription management

### 4. **UI Layer** (`ui.rs`)
- **Purpose**: Event-driven presentation logic
- **Pattern**: Event handler that responds to client events
- **Responsibilities**:
  - Status display and formatting
  - Trade data presentation
  - Error handling and user feedback

### 5. **Configuration Management** (`config.rs`)
- **Purpose**: Centralized configuration with validation
- **Pattern**: Builder pattern from CLI args
- **Structure**: Nested config structs for different concerns (WebSocket, Metrics, etc.)

## Data Flow Architecture

```
CLI Args → Config → Client + UI Controller
                      │         │
                      ▼         ▼
                 Event Bus ←────┘
                      │
                      ▼
              State Updates + UI Rendering
```

### Message Processing Pipeline:
1. **WebSocket Message** → Raw bytes
2. **Deserialization** → Typed `WebSocketMessage` enum
3. **Event Generation** → `ClientEvent` variants
4. **Event Broadcasting** → Via channels
5. **UI Handling** → Formatted output

## Design Patterns Used

### 1. **Type-Safe Message Handling**
- Uses Serde with untagged enums for WebSocket message parsing
- Custom deserializers for string-to-float conversions
- Comprehensive type definitions matching Hyperliquid's API

### 2. **Error Handling Strategy**
- Custom error types with `thiserror`
- Graceful degradation on connection failures
- Structured error propagation through Result types

### 3. **Concurrent Architecture**
- **tokio::select!** for graceful shutdown
- Separate async tasks for client and UI
- Non-blocking message processing

### 4. **Plugin Architecture**
- Modular formatters for different output types (Table, CSV, JSON)
- Optional metrics collection with Prometheus
- Configurable logging with tracing

## Key Architectural Strengths

1. **Separation of Concerns**: Clear boundaries between networking, state, and presentation
2. **Testability**: Event-driven design allows easy mocking and testing
3. **Extensibility**: New event types and formatters can be added without changing core logic
4. **Reliability**: Robust error handling and reconnection logic
5. **Performance**: Async/await throughout, minimal allocations in hot paths
6. **Observability**: Structured logging and optional metrics collection

## Trade-offs and Considerations

**Pros:**
- Clean, maintainable architecture
- Good separation of concerns
- Robust error handling
- Easy to extend and test

**Cons:**
- Some complexity overhead from the event system
- Multiple layers of abstraction
- Could be overkill for simpler use cases

This architecture is well-suited for a production trading client that needs reliability, observability, and maintainability while handling real-time market data streams.
