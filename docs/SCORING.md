# Scoring contract

`ExactText` compares the reference and response after only two transport-level
normalizations: CRLF/CR line endings become LF, and Unicode is converted to NFC.
Whitespace, capitalization, and punctuation remain significant.

`ExactWords` compares case-sensitive Unicode word sequences. Punctuation is
ignored, and straight and curly apostrophes inside words are treated alike.

Word and character error rates use Levenshtein alignment. Word accuracy is
`max(0, 1 - WER)`; the clamp prevents heavily over-generated answers from
producing negative accuracy.

An exact match for another catalogued translation of the same passage is
classified as `translation_confusion`. A merely closer alternative translation
is reported diagnostically but is not enough to assert confusion.

The translation-contamination rate is deliberately conservative. Its denominator
is the number of produced word tokens that do not align exactly with the requested
edition. A wrong produced token is "explained" when it aligns exactly in at least
one other available translation. Deleted target words are not attributed to a
different translation because the model did not produce a competing token.

Refusal detection is a transparent phrase heuristic. It is a useful operational
metric, not a semantic classifier, and every raw response remains independently
auditable.
