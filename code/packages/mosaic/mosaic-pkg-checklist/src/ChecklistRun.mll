// ChecklistRun.mll — layout.
//
//   Column [ checklist-run ]
//     Text [ checklist-title ]      (heading)
//     Text [ checklist-progress ]
//     For ( each: slot: rows, as: row, index: i )
//       Row [ checklist-row ]
//         If (row[1]) Text [ checklist-indent ]
//         If (row[3])                  — a question
//           Text [ checklist-question ]
//           Yes button (selected when row[4]) · No button (selected when row[5])
//         Else                         — a check item
//           HostCheckbox, checked (and muted) when row[4]
//     Row [ checklist-actions ]
//       If (complete-label) HostButton · If (abandon-label) HostButton
//
// Selected and unselected states are separate parts (as in the toolkit's
// SegmentedControl) so every backend can style them without relying on native
// button-state support; `selected` still reports the kernel state (UI86).
// Every conditional is a truthy test on a row marker — no string comparison.

layout ChecklistRun {
  Column [ checklist-run ] {
    Text [ checklist-title ] (
      content : slot: title ,
      a11y-role : heading
    )
    Text [ checklist-progress ] ( content : slot: progress-label )
    For ( each: slot: rows , as: row , index: i ) {
      Row [ checklist-row ] {
        If ( when: ( row[1] ) ) {
          Text [ checklist-indent ] ( content : ( row[1] ) )
        }
        If ( when: ( row[3] ) ) {
          Row [ checklist-decision ] {
            Text [ checklist-question ] ( content : ( row[2] ) )
            If ( when: ( row[4] ) ) {
              HostButton [ checklist-yes-on ] (
                label : slot: yes-label ,
                selected : true ,
                onClick : emit: onAnswerYes
              )
            }
            Else {
              HostButton [ checklist-yes ] (
                label : slot: yes-label ,
                selected : false ,
                onClick : emit: onAnswerYes
              )
            }
            If ( when: ( row[5] ) ) {
              HostButton [ checklist-no-on ] (
                label : slot: no-label ,
                selected : true ,
                onClick : emit: onAnswerNo
              )
            }
            Else {
              HostButton [ checklist-no ] (
                label : slot: no-label ,
                selected : false ,
                onClick : emit: onAnswerNo
              )
            }
          }
        }
        Else {
          // The platform's own checkbox. `onToggle ( index : number )`
          // carries this row's index (UI29-2 §2.1.1). A ticked item is its
          // own part so it can be muted.
          If ( when: ( row[4] ) ) {
            HostCheckbox [ checklist-item-done ] (
              checked : true ,
              label : ( row[2] ) ,
              onToggle : emit: onToggle
            )
          }
          Else {
            HostCheckbox [ checklist-item ] (
              checked : false ,
              label : ( row[2] ) ,
              onToggle : emit: onToggle
            )
          }
        }
      }
    }
    Row [ checklist-actions ] {
      If ( when: slot: complete-label ) {
        HostButton [ checklist-complete ] (
          label : slot: complete-label ,
          onClick : emit: onComplete
        )
      }
      If ( when: slot: abandon-label ) {
        HostButton [ checklist-abandon ] (
          label : slot: abandon-label ,
          onClick : emit: onAbandon
        )
      }
    }
  }
}
