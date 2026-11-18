# Reuse the lexer cursor in compact line parsing

Hook-free `LineParser` instances now keep a private lexer cursor as well as
their token buffer. Each parser remains single-owner; hook-bearing calls retain
the owning lexer path so hooks may retain or replace their input safely.
