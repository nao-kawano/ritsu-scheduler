// Copyright 2026 Naoyuki Kawano
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
// =============================================================================
//!
//! Message for Ritsu.
//!

mod message;

// export.
pub use message::CLIENT_ID_MAX;
pub use message::MESSAGE_LEN_MAX;
pub use message::MSG_ID_MAX;
pub use message::Message;
pub use message::MessageType;
pub use message::PROTOCOL_VERSION;
pub use message::ParseError;
