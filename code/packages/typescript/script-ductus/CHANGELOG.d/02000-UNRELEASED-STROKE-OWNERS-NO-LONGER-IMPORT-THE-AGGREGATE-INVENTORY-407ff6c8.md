### Unreleased — stroke owners no longer import the aggregate inventory

Japanese and Arabic-family stroke owners now read their canonical sharded
inventories directly instead of importing the aggregate `SCRIPTS` registry.
This preserves their source lookup while allowing browser consumers to keep
the authored pen-path registry behind a dynamic handwriting boundary.
