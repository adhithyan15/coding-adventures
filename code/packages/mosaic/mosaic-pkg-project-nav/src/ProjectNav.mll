// ProjectNav.mll — layout for the nested-project tree + add-project
// composer.
//
// Extracted verbatim from TaskApp's own rail block — same part names,
// same structure, same styling in the matching .msl themes. This is a
// refactor, not a redesign; see code/specs/task-app-project-nav-v1.md.

layout ProjectNav {
  Column [ nav-root ] {
    Text [ rail-label ] ( content : slot: nav-title )
    Column [ rail-projects ] {
      For ( each: slot: project-rows , as: p , index: pi ) {
        Row [ rail-row ] {
          // A nested project gets a leading glyph; a top-level one has an
          // empty indent cell and renders nothing, keeping the row flush
          // left.
          If ( when: ( p[2] ) ) {
            Text [ project-indent ] ( content : ( p[2] ) )
          }
          If ( when: ( p[1] ) ) {
            HostButton [ project-on ] ( label : ( p[0] ) , onClick : emit: onSelectProject )
          }
          Else {
            HostButton [ project-off ] ( label : ( p[0] ) , onClick : emit: onSelectProject )
          }
        }
      }
    }

    Row [ rail-composer ] {
      HostInput [ project-input ] (
        value : slot: new-project-name ,
        placeholder : "New project" ,
        onChange : emit: onNewProjectNameChange
      )
      HostButton [ project-add ] ( label : "+" , onClick : emit: onAddProject )
    }
    HostButton [ project-sub ] ( label : "+ Sub-project" , onClick : emit: onAddSubproject )
  }
}
