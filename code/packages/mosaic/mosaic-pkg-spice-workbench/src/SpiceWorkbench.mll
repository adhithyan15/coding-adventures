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
      For ( each: slot: diagnostic-rows , as: diagnostic , index: diagnostic-index ) {
        Text [ diagnostic-row ] ( content : ( diagnostic ) )
      }
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
        HostTable [ result-table ] {
          HostTableHead {
            Row [ result-header-row ] {
              For ( each: slot: result-columns , as: column , index: column-index ) {
                Text [ result-header ] ( content : ( column ) )
              }
            }
          }
          HostTableBody {
            For ( each: slot: result-rows , as: row , index: row-index ) {
              Row [ result-row ] {
                For ( each: row , as: value , index: value-index ) {
                  Text [ result-cell ] ( content : ( value ) )
                }
              }
            }
          }
        }
        Column [ waveforms ] {
          Text [ waveform-label ] ( content : slot: waveform-label )
          Row [ waveform-options ] {
            For ( each: slot: waveform-rows , as: waveform , index: waveform-index ) {
              HostButton [ waveform-option ] (
                label : ( waveform ) ,
                onClick : emit: onSelectWaveform
              )
            }
          }
          Text [ selected-waveform ] ( content : slot: selected-waveform-label )
          Text [ waveform-axis ] ( content : slot: waveform-axis-label )
          Stack [ waveform-plot ] {
            Path [ waveform-axis-x ] ( kind: line , x1: 28 , y1: 188 , x2: 344 , y2: 188 )
            Path [ waveform-axis-y ] ( kind: line , x1: 28 , y1: 20 , x2: 28 , y2: 188 )
            For ( each: slot: waveform-segments , as: segment , index: segment-index ) {
              Path [ waveform-segment ] (
                kind: line ,
                x1: ( segment[0] ) ,
                y1: ( segment[1] ) ,
                x2: ( segment[2] ) ,
                y2: ( segment[3] )
              )
            }
          }
        }
        Text [ raw-result-label ] ( content : slot: raw-result-label )
        Text [ result-output ] ( content : slot: result-text )
      }
    }
  }
}
