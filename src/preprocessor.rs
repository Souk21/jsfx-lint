use std::ops::Range;

use regex::Regex;

use crate::file::File;

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Chunk {
    pub raw: Range<usize>,
    pub processed: Range<usize>,
    pub epilogue_len: usize,
}

/// Preprocess `source` and return `(processed_source, chunks)`.
pub fn preprocess(source: &str) -> (String, Vec<Chunk>) {
    // This function preprocess file chunk by chunk, to be able to know the delta of each chunk.
    // Each pass pre-process from the beginning of the file to the beginning of next chunk, or EOF if it's the last chunk.
    // e.g. if there are 3 pp blocks in a file, `eel_pp` will be called 3 times.
    let mut chunks: Vec<Chunk> = Vec::new();
    let pp_ranges = collect_preprocessor_ranges(source);
    let mut final_pass = None;
    for i in 0..pp_ranges.len() {
        let pp_range = &pp_ranges[i];
        let next_pp_range = pp_ranges.get(i + 1);
        // Preprocess only until the start of the next chunk (or EOF)
        let to_process_end = next_pp_range.map_or(source.len(), |next_chunk| next_chunk.start);
        // What's between the end of the chunk and the beginning of the next chunk (or EOF)
        let epilogue = &source[pp_range.end..to_process_end];
        let to_process = &source[..to_process_end];
        let processed = File::preprocess_str(to_process);
        // Will be 0 if the epilogue is longer than the processed string
        let epilogue_start_in_processed = processed.len().saturating_sub(epilogue.len());
        let found = &processed[epilogue_start_in_processed..] == epilogue;
        let prev_chunk = chunks.last();
        let new_start = prev_chunk.map_or(pp_range.start, |prev_chunk| {
            prev_chunk.processed.end + prev_chunk.epilogue_len
        });
        if found {
            chunks.push(Chunk {
                raw: pp_range.clone(),
                processed: new_start..epilogue_start_in_processed,
                epilogue_len: epilogue.len(),
            });
        } else {
            chunks.push(Chunk {
                raw: pp_range.clone(),
                processed: new_start..new_start,
                epilogue_len: 0,
            });
        }
        final_pass = Some(processed);
    }
    (final_pass.unwrap_or_else(|| String::from(source)), chunks)
}

fn collect_preprocessor_ranges(source: &str) -> Vec<Range<usize>> {
    // "(?s)" makes "." match newlines
    let regex = Regex::new(r"(?s)<\?.*?\?>").unwrap();
    regex.find_iter(source).map(|m| m.range()).collect()
}

#[cfg(test)]
mod tests {
    use crate::preprocessor::{collect_preprocessor_ranges, preprocess};
    use indoc::indoc;

    #[test]
    fn single_line() {
        let source = "1234<?12?>123";
        let chunks = collect_preprocessor_ranges(source);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].start, 4);
        assert_eq!(chunks[0].end, 10);
    }

    #[test]
    fn single_line_multiple_blocks() {
        let source = "1234<?12?>123<?12?>123";
        let chunks = collect_preprocessor_ranges(source);
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].start, 4);
        assert_eq!(chunks[0].end, 10);
        assert_eq!(chunks[1].start, 13);
        assert_eq!(chunks[1].end, 19);
    }

    #[test]
    fn single_block() {
        let source = indoc! {"
            @init
            <? something
            ?>
        "};
        let chunks = collect_preprocessor_ranges(source);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].start, 6);
        assert_eq!(chunks[0].end, 21);
    }
    #[test]
    fn multiple_blocks() {
        let source = indoc! {"
            @init
            <? something
            ?>
            a = 2;
            <? something
            else ?>
        "};
        let chunks = collect_preprocessor_ranges(source);
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].start, 6);
        assert_eq!(chunks[0].end, 21);
        assert_eq!(chunks[1].start, 29);
        assert_eq!(chunks[1].end, 49);
    }

    #[test]
    fn no_blocks() {
        let source = indoc! {"
            @init
            a = 2;
        "};
        let chunks = collect_preprocessor_ranges(source);
        assert_eq!(chunks.len(), 0);
    }

    #[test]
    fn basic_preprocessing() {
        let prelude = "a += 1;\nb += 2;\n";
        let block = r#"<? printf("c = 3;") ?>"#;
        let epilogue = "\nd = 4;";
        let source = format!("{prelude}{block}{epilogue}");
        let (processed, chunks) = preprocess(&source);
        assert_eq!(processed, "a += 1;\nb += 2;\nc = 3;\nd = 4;");
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].raw.start, prelude.len());
        assert_eq!(chunks[0].raw.end, prelude.len() + block.len());
        assert_eq!(chunks[0].processed.start, prelude.len());
        assert_eq!(chunks[0].processed.end, prelude.len() + "c = 3;".len());
    }

    #[test]
    fn lonely_block() {
        let source = r#"<? printf("a = 1;"); ?>"#;
        let (processed, chunks) = preprocess(source);
        assert_eq!(processed, "a = 1;");
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].raw.start, 0);
        assert_eq!(chunks[0].raw.end, source.len());
        assert_eq!(chunks[0].processed.start, 0);
        assert_eq!(chunks[0].processed.end, "a = 1;".len());
    }

    #[test]
    fn multiple_blocks_preprocessing() {
        let prelude = "a += 1;\nb += 2;\n";
        let block1 = r#"<? printf("c = 3;") ?>"#;
        let inter = "\nd = 4;\n";
        let block2 = r#"<? printf("e = 5;") ?>"#;
        let epilogue = "\nf = 6;";
        let source = format!("{prelude}{block1}{inter}{block2}{epilogue}");
        let (processed, chunks) = preprocess(&source);
        assert_eq!(
            processed,
            "a += 1;\nb += 2;\nc = 3;\nd = 4;\ne = 5;\nf = 6;"
        );
        let block_1_expected = "c = 3;";
        let block_2_expected = "e = 5;";
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].raw.start, prelude.len());
        assert_eq!(chunks[0].raw.end, prelude.len() + block1.len());
        assert_eq!(chunks[0].processed.start, prelude.len());
        assert_eq!(
            chunks[0].processed.end,
            prelude.len() + block_1_expected.len()
        );
        assert_eq!(
            chunks[1].raw.start,
            prelude.len() + block1.len() + inter.len()
        );
        assert_eq!(
            chunks[1].raw.end,
            prelude.len() + block1.len() + inter.len() + block2.len()
        );
        assert_eq!(
            chunks[1].processed.start,
            prelude.len() + block_1_expected.len() + inter.len()
        );
        assert_eq!(
            chunks[1].processed.end,
            prelude.len() + block_1_expected.len() + inter.len() + block_2_expected.len()
        );
    }

    #[test]
    fn suppress() {
        let prelude = "a = 1;\n";
        let block = "<? _suppress = 1; ?>";
        let epilogue = "\nb = 2;";
        let source = format!("{prelude}{block}{epilogue}");
        let (processed, chunks) = preprocess(&source);
        assert_eq!(processed, "a = 1;\n");
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].raw.start, prelude.len());
        assert_eq!(chunks[0].raw.end, prelude.len() + block.len());
        assert_eq!(chunks[0].processed.start, prelude.len());
        assert_eq!(chunks[0].processed.end, prelude.len());
    }

    #[test]
    fn suppress_end() {
        let prelude = "a = 1;\n";
        let block1 = "<? _suppress = 1; ?>";
        let inter = "\nb = 2;\n";
        let block2 = "<? _suppress = 0; ?>";
        let epilogue = "\nc = 3;";
        let source = format!("{prelude}{block1}{inter}{block2}{epilogue}");
        let (processed, chunks) = preprocess(&source);
        assert_eq!(processed, "a = 1;\n\nc = 3;");
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].raw.start, prelude.len());
        assert_eq!(chunks[0].raw.end, prelude.len() + block1.len());
        assert_eq!(chunks[0].processed.start, prelude.len());
        assert_eq!(chunks[0].processed.end, prelude.len());
        assert_eq!(
            chunks[1].raw.start,
            prelude.len() + block1.len() + inter.len()
        );
        assert_eq!(
            chunks[1].raw.end,
            prelude.len() + block1.len() + inter.len() + block2.len()
        );
        assert_eq!(chunks[1].processed.start, prelude.len());
        assert_eq!(chunks[1].processed.end, prelude.len());
    }

    #[test]
    fn lots_of_lines() {
        let prelude = "a = 1;\n";
        let block = r#"<? loop(100, printf("b = 2;\n")); ?>"#;
        let epilogue = "\nc = 3;";
        let source = format!("{prelude}{block}{epilogue}");
        let expected_block = "b = 2;\n".repeat(100);
        let (processed, chunks) = preprocess(&source);
        assert_eq!(processed, format!("{prelude}{expected_block}{epilogue}"));
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].raw.start, prelude.len());
        assert_eq!(chunks[0].raw.end, prelude.len() + block.len());
        assert_eq!(chunks[0].processed.start, prelude.len());
        assert_eq!(
            chunks[0].processed.end,
            prelude.len() + expected_block.len()
        );
    }
}
