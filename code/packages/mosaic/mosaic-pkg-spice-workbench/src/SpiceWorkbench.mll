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
    Column [ schematic ] {
      Text [ schematic-label ] ( content : slot: schematic-label )
      Text [ schematic-title ] ( content : slot: schematic-title )
      Row [ schematic-palette ] {
        For ( each: slot: schematic-palette , as: kind , index: kind-index ) {
          HostButton [ schematic-palette-item ] (
            label : ( kind ) ,
            onClick : emit: onPlaceSchematicComponent
          )
        }
      }
      Text [ schematic-analysis-label ] ( content : slot: schematic-analysis-label )
      Row [ schematic-analysis-controls ] {
        For ( each: slot: schematic-analysis-controls , as: analysis , index: analysis-index ) {
          HostButton [ schematic-analysis-control ] (
            label : ( analysis ) ,
            onClick : emit: onSelectSchematicAnalysis
          )
        }
      }
      Text [ selected-schematic-analysis ] ( content : slot: selected-schematic-analysis-label )
      Column [ schematic-analysis-configuration ] {
        Text [ schematic-analysis-configuration-label ] ( content : slot: schematic-analysis-configuration-label )
        Text [ schematic-analysis-source-label ] ( content : slot: schematic-analysis-source-label )
        Row [ schematic-analysis-source-options ] {
          For ( each: slot: schematic-analysis-source-options , as: source , index: source-index ) {
            HostButton [ schematic-analysis-source-option ] (
              label : ( source ) ,
              onClick : emit: onSelectSchematicAnalysisSource
            )
          }
        }
        Text [ selected-schematic-analysis-source ] ( content : slot: selected-schematic-analysis-source-label )
        Text [ schematic-analysis-parameter-one-label ] ( content : slot: schematic-analysis-parameter-one-label )
        HostInput [ schematic-analysis-parameter-one-input ] (
          value : slot: schematic-analysis-parameter-one-value ,
          disabled : slot: schematic-analysis-parameter-one-disabled ,
          onChange : emit: onSchematicAnalysisParameterOneChange
        )
        Text [ schematic-analysis-parameter-two-label ] ( content : slot: schematic-analysis-parameter-two-label )
        HostInput [ schematic-analysis-parameter-two-input ] (
          value : slot: schematic-analysis-parameter-two-value ,
          disabled : slot: schematic-analysis-parameter-two-disabled ,
          onChange : emit: onSchematicAnalysisParameterTwoChange
        )
        Text [ schematic-analysis-parameter-three-label ] ( content : slot: schematic-analysis-parameter-three-label )
        HostInput [ schematic-analysis-parameter-three-input ] (
          value : slot: schematic-analysis-parameter-three-value ,
          disabled : slot: schematic-analysis-parameter-three-disabled ,
          onChange : emit: onSchematicAnalysisParameterThreeChange
        )
      }
      Text [ schematic-grid-label ] ( content : slot: schematic-grid-label )
      Stack [ schematic-grid ] {
        For ( each: slot: schematic-grid-lines , as: line , index: line-index ) {
          Path [ schematic-grid-line ] (
            kind: line ,
            x1: ( line[0] ) ,
            y1: ( line[1] ) ,
            x2: ( line[2] ) ,
            y2: ( line[3] )
          )
        }
        For ( each: slot: schematic-wire-segments , as: segment , index: segment-index ) {
          Path [ schematic-wire-segment ] (
            kind: line ,
            x1: ( segment[0] ) ,
            y1: ( segment[1] ) ,
            x2: ( segment[2] ) ,
            y2: ( segment[3] )
          )
        }
        For ( each: slot: schematic-terminal-points , as: point , index: point-index ) {
          Path [ schematic-terminal ] (
            kind: circle ,
            cx: ( point[0] ) ,
            cy: ( point[1] ) ,
            r: 5
          )
        }
      }
      Row [ schematic-components ] {
        For ( each: slot: schematic-rows , as: component , index: component-index ) {
          HostButton [ schematic-component ] (
            label : ( component ) ,
            onClick : emit: onSelectSchematicComponent
          )
        }
      }
      Text [ selected-schematic ] ( content : slot: selected-schematic-label )
      Column [ schematic-properties ] {
        Text [ schematic-properties-label ] ( content : slot: schematic-properties-label )
        Text [ selected-schematic-kind ] ( content : slot: selected-schematic-kind-label )
        Text [ schematic-value-label ] ( content : slot: schematic-value-label )
        HostInput [ schematic-value-input ] (
          value : slot: schematic-value ,
          placeholder : slot: schematic-value-placeholder ,
          disabled : slot: schematic-value-disabled ,
          onChange : emit: onSchematicValueChange
        )
      }
      Text [ route-schematic ] ( content : slot: route-schematic-label )
      Row [ schematic-route-targets ] {
        For ( each: slot: schematic-rows , as: component , index: component-index ) {
          HostButton [ schematic-route-target ] (
            label : ( component ) ,
            onClick : emit: onRouteToSchematicComponent
          )
        }
      }
      HostButton [ synchronize-schematic ] (
        label : slot: synchronize-schematic-label ,
        onClick : emit: onSynchronizeSchematic
      )
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
