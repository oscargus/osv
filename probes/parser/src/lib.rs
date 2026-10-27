#[cfg(test)]
mod tests {
    use vhdl_syntax::{
        parser,
        standard::VHDLStandard,
        syntax::{AstNode, DesignFileSyntax, NodeKind},
    };

    fn assert_lossless(source: &[u8], expected_units: &[NodeKind]) -> DesignFileSyntax {
        let (design, diagnostics) = parser::parse_with_standard(VHDLStandard::VHDL2008, source);
        assert!(diagnostics.is_empty(), "{diagnostics:#?}");
        assert_eq!(design.raw().range(), 0..source.len());

        let units: Vec<_> = design
            .children()
            .filter_map(|unit| unit.first_child().map(|node| node.kind()))
            .collect();
        assert_eq!(units, expected_units);

        let mut reproduced = Vec::new();
        design.raw().write_to(&mut reproduced).unwrap();
        assert_eq!(reproduced, source);
        design
    }

    #[test]
    fn parses_entity_and_architecture() {
        assert_lossless(
            b"entity probe is\nend entity;\n\
             architecture sim of probe is\nbegin\nend architecture;\n",
            &[NodeKind::EntityDeclaration, NodeKind::ArchitectureBody],
        );
    }

    #[test]
    fn parses_package() {
        assert_lossless(
            b"package probe_types is\n\
             type state is (idle, running);\n\
             end package;\n",
            &[NodeKind::PrimaryUnitPackageDeclaration],
        );
    }

    #[test]
    fn parses_vhdl_2008_process_all() {
        assert_lossless(
            b"entity probe is end entity;\n\
              architecture sim of probe is\n\
                signal observed : boolean;\n\
              begin\n\
                process(all) begin observed <= false; end process;\n\
              end architecture;\n",
            &[NodeKind::EntityDeclaration, NodeKind::ArchitectureBody],
        );
    }

    #[test]
    fn preserves_latin1_bytes_and_node_offsets() {
        let source = b"-- caf\xe9\r\nentity latin_probe is\r\nend entity;\r\n";
        let design = assert_lossless(source, &[NodeKind::EntityDeclaration]);
        let entity = design.children().next().unwrap().first_child().unwrap();
        assert_eq!(entity.text_range().start, 9);
        assert_eq!(&source[entity.text_range().start..][..6], b"entity");
    }

    #[test]
    fn leaves_name_and_type_checks_to_semantic_analysis() {
        assert_lossless(
            b"entity probe is end entity;\n\
              architecture sim of probe is\n\
                signal missing : undeclared_type;\n\
                signal wrong : boolean := 42;\n\
              begin end architecture;\n",
            &[NodeKind::EntityDeclaration, NodeKind::ArchitectureBody],
        );
    }

    #[test]
    fn reports_malformed_input() {
        let source = b"entity broken is\n";
        let (design, diagnostics) = parser::parse_with_standard(VHDLStandard::VHDL2008, source);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].span(), &(16..16));
        assert!(matches!(
            diagnostics[0].err(),
            parser::error::SyntaxErrKind::Expected(_)
        ));
        assert_eq!(
            parser::error::display_errors(&diagnostics),
            "16..16 expected EntityDeclarationEpilogue"
        );

        let mut reproduced = Vec::new();
        design.raw().write_to(&mut reproduced).unwrap();
        assert_eq!(reproduced, source);
        println!("{}", parser::error::display_errors(&diagnostics));
    }

    #[test]
    fn reports_nonempty_half_open_byte_spans() {
        let source = b"entity broken is\n\x00\nend entity;\n";
        let (_, diagnostics) = parser::parse_with_standard(VHDLStandard::VHDL2008, source);
        assert!(
            diagnostics.iter().any(|error| error.span() == &(17..18)),
            "{diagnostics:#?}"
        );
        assert_eq!(&source[17..18], b"\x00");
    }

    #[test]
    fn rejects_vhdl_2019_mode_views_in_2008() {
        let source = b"package revision_probe is\n\
                         type pair is record value : bit; end record;\n\
                         view read_only of pair is value : in; end view;\n\
                       end package;\n";
        let (_, diagnostics) = parser::parse_with_standard(VHDLStandard::VHDL2008, source);
        assert!(!diagnostics.is_empty());
    }
}
