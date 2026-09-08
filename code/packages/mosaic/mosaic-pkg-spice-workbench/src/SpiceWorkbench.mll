layout SpiceWorkbench {
  Column [ workbench ] {
    Row [ workbench-header ] {
      Text [ workbench-title ] ( content : slot: workbench-title )
      Text [ workbench-mode ] ( content : slot: mode-label )
    }
    Column [ deck-editor ] {
      Text [ netlist-label ] ( content : slot: netlist-label )
      Input [ netlist-input ] (
        multiline : true ,
        value : slot: netlist-text ,
        placeholder : slot: netlist-placeholder ,
        onChange : emit: onNetlistChange
      )
      Row [ workbench-actions ] {
        HostButton [ inspect-button ] ( label : slot: inspect-label , onClick : emit: onInspect )
        HostButton [ run-button ] ( label : slot: run-label , onClick : emit: onRun )
      }
      Text [ diagnostics-label ] ( content : slot: diagnostics-label )
      Text [ diagnostics ] ( content : slot: diagnostics )
    }
    Row [ workbench-body ] {
      Column [ analyses ] {
        Text [ analysis-label ] ( content : slot: analysis-label )
        For ( each: slot: analysis-rows , as: analysis , index: analysis-index ) {
          HostButton [ analysis-option ] (
            label : ( analysis[0] ) ,
            onClick : emit: onSelectAnalysis
          )
        }
        Text [ selected-analysis ] ( content : slot: selected-analysis-label )
      }
      Column [ results ] {
        Text [ result-label ] ( content : slot: result-label )
        Text [ result-output ] ( content : slot: result-text )
      }
    }
  }
}
