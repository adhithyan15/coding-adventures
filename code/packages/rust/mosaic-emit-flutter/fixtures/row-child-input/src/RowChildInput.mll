// Every text field here sits BELOW a Row child rather than in the Row itself,
// which is where Flutter measures it with an unbounded width. NoteEditor
// threw 27 layout assertions from the first shape when first pumped.
//
//   editor-body   NoteEditor's shape: a list column beside a Box that holds
//                 a label and two fields.
//   limits        DeckOptionsPanel's shape: two labelled number fields.
//   outer > inner a Row nested inside a Row child, bounded only once the
//                 outer child is.
layout RowChildInput {
  Column [ root ] {
    Row [ editor-body ] {
      Column [ field-list ] {
        For ( each: slot: field-names , as: name ) {
          Text [ field-name ] ( content : name )
        }
      }
      Box [ focused-field ] {
        Text [ focused-label ] ( content : slot: field-label )
        HostInput [ focused-input ] (
          value : slot: field-value ,
          placeholder : "Front" ,
          onChange : emit: onFieldValueChange
        )
        HostInput [ tags-input ] ( value : "" , placeholder : "tags" )
      }
    }
    Row [ limits ] {
      Column [ new-cards ] {
        Text [ new-cards-label ] ( content : "New cards/day" )
        HostNumberInput [ new-cards-input ] ( placeholder : "20" )
      }
      Column [ reviews ] {
        Text [ reviews-label ] ( content : "Reviews/day" )
        HostNumberInput [ reviews-input ] ( placeholder : "200" )
      }
    }
    Row [ outer ] {
      Box [ outer-cell ] {
        Row [ inner ] {
          Text [ inner-label ] ( content : "Search" )
          Box [ inner-cell ] {
            HostInput [ search-input ] ( value : "" , placeholder : "Find" )
          }
        }
      }
    }
  }
}
