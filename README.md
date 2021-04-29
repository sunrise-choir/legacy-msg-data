# SSB Legacy MSG Data

Rust implementation of the [ssb legacy data format](https://spec.scuttlebutt.nz/feed/datamodel.html).

Please be aware that this fork deviates significantly from the base repository, primarily in the strictness of the deserialization for message content values.

While the base repository allows deserialization of any valid JSON value for the `content` field of a message - provided it is in the form of a map (key-value pair), this implementation will fail if the `content` field is not a string or map (dictionary). The change has been made to increase validation consistency with the JavaScript implementation of SSB.
