(cd ../pixel-container && npm ci --quiet)
(cd ../paint-instructions && npm ci --quiet)
(cd ../paint-vm && npm ci --quiet)
(cd ../paint-vm-svg && npm ci --quiet)
npm ci --quiet
# HL-C82: the grammar-slot inventory and Spanish cell ledger are generated.
# Fail the build if the committed files drift from the generator, the same
# contract the check:books / check:modality targets already enforce.
python3 ../../../learning/human-languages/data/generate_grammar_cells.py --check
npx vitest run --coverage
