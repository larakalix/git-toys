import js from '@eslint/js'
import ts from 'typescript-eslint'
import vue from 'eslint-plugin-vue'
export default ts.config(js.configs.recommended,...ts.configs.recommended,...vue.configs['flat/recommended'],{files:['**/*.vue'],languageOptions:{parserOptions:{parser:ts.parser}},rules:{'no-undef':'off','vue/multiline-html-element-content-newline':'off','vue/multi-word-component-names':'off','vue/html-self-closing':'off','vue/max-attributes-per-line':'off','vue/singleline-html-element-content-newline':'off','vue/html-indent':'off','vue/html-closing-bracket-newline':'off','vue/first-attribute-linebreak':'off','vue/attributes-order':'off'}})
