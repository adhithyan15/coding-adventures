// RecordList.mll — layout for the RecordList.
//
//   Column [ record-list ]
//     For ( each: slot: rows, as: row, index: i )
//       If (row[1])                 Text [ record-list-heading ]   (heading)
//       If (row[0] == selectedKey)  <row, "-selected" parts>
//       Else                        <row>
//
//   row = Column [ record-list-row ]
//           Row [ record-list-line ]
//             HostButton [ record-list-title ]   (label: title, onClick: onSelect)
//             If (row[4]) Text [ record-list-meta ]
//             If (row[5]) Text [ record-list-badge ]
//           If (row[3]) Text [ record-list-subtitle ]
//
// Why the button is the title, not the whole row
// ----------------------------------------------
// The obvious design is one HostButton per row with the fields inside it. It
// does not work today: children nested in a HostButton are dropped on seven of
// eight backends (React, WebComponent, HTML, SwiftUI, Compose, Flutter and Qt
// emit an empty button; only XAML keeps them), and the degradation report stays
// clean. Children pass-through is the unwritten UI29-2 spec that also blocks the
// toolkit's Card. So the title is the button — one tab stop, one activation,
// named by its own label on every backend — and the other fields sit beside and
// below it. When UI29-2 lands the whole row can become the target without
// changing this component's interface.
//
// The selected row gets its own parts, as in ListGroup, so every backend can
// style it without relying on native button-state support; `selected` still
// reports the kernel state (UI86) to assistive technology.
//
// The heading is a Text with the heading role, drawn inside the same For
// iteration just above the first row of its group, so groups need no second
// list and no nested For (which would lose the row index, UI37).
//
// `selected-key` remains the public kebab-case slot; the expression uses
// selectedKey because emitters expose kebab slots as camel-case identifiers
// in predicates.

layout RecordList {
  Column [ record-list ] {
    For ( each: slot: rows , as: row , index: i ) {
      If ( when: ( row[1] ) ) {
        Text [ record-list-heading ] (
          content : ( row[1] ) ,
          a11y-role : heading
        )
      }
      If ( when: ( row[0] == selectedKey ) ) {
        Column [ record-list-row-selected ] {
          Row [ record-list-line-selected ] {
            HostButton [ record-list-title-selected ] (
              label : ( row[2] ) ,
              selected : true ,
              onClick : emit: onSelect
            )
            If ( when: ( row[4] ) ) {
              Text [ record-list-meta-selected ] ( content : ( row[4] ) )
            }
            If ( when: ( row[5] ) ) {
              Text [ record-list-badge-selected ] ( content : ( row[5] ) )
            }
          }
          If ( when: ( row[3] ) ) {
            Text [ record-list-subtitle-selected ] ( content : ( row[3] ) )
          }
        }
      }
      Else {
        Column [ record-list-row ] {
          Row [ record-list-line ] {
            HostButton [ record-list-title ] (
              label : ( row[2] ) ,
              selected : false ,
              onClick : emit: onSelect
            )
            If ( when: ( row[4] ) ) {
              Text [ record-list-meta ] ( content : ( row[4] ) )
            }
            If ( when: ( row[5] ) ) {
              Text [ record-list-badge ] ( content : ( row[5] ) )
            }
          }
          If ( when: ( row[3] ) ) {
            Text [ record-list-subtitle ] ( content : ( row[3] ) )
          }
        }
      }
    }
  }
}
