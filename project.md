# OSV - VHDL simulator in Rust

This project is a VHDL simulator written in Rust called osv. It relies on the vhdl_syntax crate for parsing VHDL 2008. Only VHDL 2008 is supported. osv should be possible to compile on any major platform (at least x86-64 and arm64) and operating system (at least linux, windows, and macos).

The goal is to have an event-driven compiled simulator where the simulation steps are efficiently dispatched to small functions.
The functions should be specialized for data types etc.
All IEEE 1076-2008 standard-library packages should have native types and functions so that their VHDL package bodies are not required: ieee.std_logic_1164, ieee.numeric_std, ieee.numeric_std_unsigned, ieee.numeric_bit, ieee.numeric_bit_unsigned, ieee.math_real, ieee.math_complex, ieee.fixed_float_types, ieee.fixed_generic_pkg, ieee.fixed_pkg, ieee.float_generic_pkg, ieee.float_pkg, and the ieee.std_logic_textio compatibility package.
The ieee.ieee_std_context and ieee.ieee_bit_context context declarations should also be supported, including their transitive declarations and visibility rules.
All VHDL 2008 std packages (std.standard, std.textio, and std.env) should be supported natively as well.
Coverage includes every applicable overload, alias, implicit predefined operation, generic-package policy, and text-I/O signature. VHDL 2019 additions, Synopsys compatibility packages, and separately standardized VITAL packages are outside this IEEE 1076-2008 scope unless explicitly added later.
One may consider adding foreign function calls instead of the package bodies or simply skip the package bodies and provide direct bindings for those functions.

Values should be efficiently represented, where, e.g., unsigned should be represented as unsigned integers (or BigUInt for more than 64 bits) if consisting of only 0 and 1, or as a string or vector for non 0/1-values.

The simulator should follow a three-step approach: analysis, elaboration, and simulation.
Analysis should simplify and specialize dispatch as much as possible, as well as doing width and type checks to the furthest possibility. There should be a serialized artifact for each primary unit.
Elaborate should take a top-level enity name and e.g. generics into account and produce a simplified serialized artifact of the module, possibly linking to other artifacts or include them in the elaborated artifact.
Simulation should take an elaborated entity and run the simulation.

Simulation should optionally generate VCD and FST waveform files, individually or together. FST output must use the `fst_writer` library (Cargo package `fst-writer`); VCD output should use an established writer where suitable and also support de-facto extensions for e.g. strings, enums, and type names. Waveforms should include selected hierarchical signals (all by default), initial values, changes, and a timescale matching simulation resolution. Format-specific logic-state and delta-cycle limitations must be documented without changing simulation semantics.

Analyzed and elaborated artifacts should use binary serialization by default, with inspectable RON serialization of the same versioned logical schema also supported. Both formats should be loadable and convertible without reanalyzing sources, and should produce identical simulation behavior.

It should be possible to analyze files separately (although it may be OK to require that dependent files are analyzed before the current file).

Error messages should be detailed, using the ariadne crate, ideally with several levels/spans of messages, pointing out e.g. types of involved parts if there is a type mismatch etc. There should be tests for error messages as well with expected outputs stored. Make sure that these are platform independent in terms of search path separators.

Run `cargo fmt` for each chunk (no need to check first). Also run `cargo clippy` and `cargo check` and fix possible issues for each chunk.

CI should use GitHub Actions for pull requests and pushes, with formatting, compilation, Clippy, tests, native-library coverage, and VCD/FST output checks as their implementations become available. Required reference/reader tools should be pinned and missing tools must not count as a passed check. Include cross-platform checks for diagnostic path portability.

Prefer to use enum types over string parsing in serialization and elsewhere. Consider carefully if string-ids are required or if the information can be obtained from other fields.

VHDL-fixture-file-based tests are preferred, ideally with VHDL asserts to prove functionality. If required, it is also OK to inline VHDL source code in the test. Pure Rust-tests should be avoided if possible.
