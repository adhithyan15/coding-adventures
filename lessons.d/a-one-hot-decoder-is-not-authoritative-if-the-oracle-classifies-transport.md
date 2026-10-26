---
category: Cryptography & security review
---

# A one-hot decoder is not authoritative if the oracle classifies transport before its lines select the operation

The IBM System/360 gate simulator had a correct 256-line decoder, but the
public transition called the functional simulator first.  The functional
oracle's host `instruction_width` match therefore decided whether an opcode
was RR, four-byte, SS, reserved, or truncated before the gate executor ever
observed it.  Checking a one-hot line after that decision did not make the
decoder authoritative.

The repair settles one decoder before invoking the oracle.  Gate OR reductions
derive the transport width first, preserving the important rule that a
truncated reserved encoding faults as truncated; a separate gate reduction
then authorizes at most one legal operation.  The selected operation descriptor
is passed through execution and independent validation, including BXH/BXLE, so
neither layer decodes the byte again.  The oracle remains a semantic and trace
comparison, but it no longer decides which candidate is allowed to run.

When auditing a purported hardware decoder, trace authority from the first
untrusted byte through fetch width, legality, family routing, operation routing,
validation, and commit.  A decoder used only after any one of those decisions
is evidence, not authority.  Test all first bytes at every transport width,
include reserved holes whose truncation precedence differs from their final
opcode fault, and use a storage-poison witness to prove inactive candidates are
not evaluated eagerly.
