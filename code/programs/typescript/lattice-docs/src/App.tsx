import { useState, useEffect } from "react";
import { Navbar } from "./components/Navbar";
import { Hero } from "./components/Hero";
import { Features } from "./components/Features";
import { Playground } from "./components/Playground";
import { SyntaxReference } from "./components/SyntaxReference";
import { HowItWorks } from "./components/HowItWorks";
import { LanguageSupport } from "./components/LanguageSupport";
import { InstallGuide } from "./components/InstallGuide";
import { Footer } from "./components/Footer";

export type Theme = "dark" | "light";

export default function App() {
  const [theme, setTheme] = useState<Theme>(() => {
    const saved = localStorage.getItem("lattice-theme");
    if (saved === "light" || saved === "dark") return saved;
    return window.matchMedia("(prefers-color-scheme: light)").matches
      ? "light"
      : "dark";
  });

  useEffect(() => {
    document.documentElement.setAttribute("data-theme", theme);
    localStorage.setItem("lattice-theme", theme);
  }, [theme]);

  const toggleTheme = () =>
    setTheme((t) => (t === "dark" ? "light" : "dark"));

  return (
    <div className="app">
      <Navbar theme={theme} onToggleTheme={toggleTheme} />
      <main>
        <Hero />
        <Features />
        <Playground />
        <SyntaxReference />
        <HowItWorks />
        <LanguageSupport />
        <InstallGuide />
      </main>
      <Footer />
    </div>
  );
}
