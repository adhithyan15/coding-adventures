import { expect, it } from "vitest";
import { autolinkHeadings } from "../src/index.js";
it("does not create headings from fence text", () => {
 expect(autolinkHeadings({type:"document",children:[{type:"fenced_block",name:null,info:null,value:"# heading\n"}]})).toEqual([]);
});
