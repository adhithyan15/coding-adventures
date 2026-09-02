const NEW = new Set(process.argv.slice(2));
console.log("argv count", process.argv.length, "set size", NEW.size);
console.log("has KA-C67-and:", NEW.has("KA-C67-and"));
console.log([...NEW].slice(0,5));
