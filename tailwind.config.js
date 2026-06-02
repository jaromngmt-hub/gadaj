/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{js,ts,jsx,tsx}"],
  theme: {
    extend: {
      colors: {
        gadaj: {
          50: "#f5f7fa",
          100: "#eaeef3",
          200: "#cdd6e1",
          300: "#a7b5c8",
          400: "#7c8eaa",
          500: "#5c7090",
          600: "#475776",
          700: "#3a475d",
          800: "#303a4b",
          900: "#1a2230",
          950: "#0e131c",
        },
        accent: {
          50: "#fdf4ff",
          100: "#fae8ff",
          200: "#f5d0fe",
          300: "#f0abfc",
          400: "#e879f9",
          500: "#d946ef",
          600: "#c026d3",
          700: "#a21caf",
          800: "#86198f",
          900: "#701a75",
        },
      },
      fontFamily: {
        sans: ["Inter", "system-ui", "-apple-system", "BlinkMacSystemFont", "Segoe UI", "Roboto", "sans-serif"],
      },
    },
  },
  plugins: [],
};
