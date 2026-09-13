import QtQuick 2.15
import QtTest 1.15
Item {
 width: 800; height: 700
 ScaledText { id: subject; textSize: 13 }
 function surface(item, label) {
   if ((item.text === label || item.objectName === label) && item.font !== undefined) return item;
   for (var i=0; item.children && i<item.children.length; ++i) {
     var found=surface(item.children[i],label); if(found) return found;
   }
   return null;
 }
 TestCase { name: "GeneratedTypography"; when: windowShown
   function test_scaleAndRestore() {
     subject.textSize=NaN;
     var platformSize=surface(subject,"Default").font.pixelSize;
     var inputs=[13,19.5,26,NaN,Infinity,0,-1,1e100,0.1];
     var expected=[13,20,26,13,13,13,13,13,13];
     var labels=["Typography","Action","Inherited","Editor","field"];
     for(var n=0;n<inputs.length;++n) {
       subject.textSize=inputs[n];
       compare(surface(subject,"Default").font.pixelSize, n < 3 ? expected[n] : platformSize, "platform default restoration");
       for(var i=0;i<labels.length;++i) {
         var item=surface(subject,labels[i]); verify(item !== null, labels[i]);
         compare(item.font.pixelSize,expected[n],labels[i]+" at "+inputs[n]);
       }
     }
   }
 }
}
