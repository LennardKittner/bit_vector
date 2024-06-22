# bit_vector
A bit vector that supports fast rank and select

# Build / run

The main application can be build using cargo
`cargo build --release --bin bit_vector`
or build and run 
`cargo run --release --bin bit_vector -- <in_path> <out_path>`

## Features
There are also features which can be used to changes aspects of the bit_vector.
By default, the features `USE_SELECT_TABLE` and `UNIT_USIZE` are enabled.
`USE_SELECT_TABLE` decides whether a lookup table should be used to accelerate select operations.
`UNIT_USIZE` specifies the data type which should be used inside the bit_vector to store the raw data.
Other options are e.g. `UNIT_U8` or `UNIT_U16` to use `u8` and `u16` as type to store the raw data .
` cargo run --release --bin bit_vector --no-default-features --features "USE_SELECT_TABLE, UNIT_U8" -- <in_path> <out_path>`
will run the main executable with `u8` as raw data type.

## Documentation
Documentation is available and can be generated and opened with
`cargo doc --open`