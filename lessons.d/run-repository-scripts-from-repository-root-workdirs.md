# Run repository scripts from repository-root workdirs

Running `code/scripts/lessons.py` from a package working directory prepended the
package path and could not find the repository script. Repository-wide scripts
should run with the repository root as their command working directory; use a
separate package-scoped command for build-tool outputs such as JaCoCo XML.
