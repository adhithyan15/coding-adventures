## 0.351.0 - Runtime real output

Direct zero-argument ALGOL `real` procedure results now print through the
portable six-significant-digit typed-IIR formatter already used by Dartmouth
BASIC. Scalar literal and statically tracked values retain their source-spelling
path; general dynamic scalar formatting remains conservative. The shared helper
prints NaN and infinities without entering decimal-normalization loops.
