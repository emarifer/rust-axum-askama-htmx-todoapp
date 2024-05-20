const { fontFamily } = require('tailwindcss/defaultTheme');

/** @type {import('tailwindcss').Config} */

module.exports = {
  content: ["../templates/**/*.{html,js}"],
  theme: {
    extend: {
      fontFamily: {
        sans: ['Kanit', ...fontFamily.sans]
      }
    },
  },
  plugins: [
    require("daisyui")
  ],
  daisyui: {
    themes: ["dark"]
  }
}

