# A late retrieval lesson still needs an explicit prerequisite path to the atom it assesses

Curriculum reading order does not make every earlier atom transitively available.
The validator follows each lesson's declared prerequisite graph, so a late review
lesson that names only the immediately preceding content lesson can still fail
with `required atom ... is not introduced by a transitive prerequisite`. Preserve
the local sequence prerequisite, but also name the original teaching lesson (or
the latest prerequisite that transitively contains it). This keeps the review
honest when paths and optional extensions are composed independently.
