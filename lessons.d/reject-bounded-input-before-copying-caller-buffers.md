# Reject bounded input before copying caller buffers

The first Kotlin implementation copied the entire caller buffer before the
delegated DER-TLV decoder enforced `maxInputLength`, allowing oversized input
to force an avoidable allocation. Pass caller input directly to the bounded
framing layer, then snapshot only the successfully validated element views.
