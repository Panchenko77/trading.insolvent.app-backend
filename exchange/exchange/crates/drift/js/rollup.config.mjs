import typescript from "@rollup/plugin-typescript";
import { nodeResolve } from '@rollup/plugin-node-resolve';
import commonjs from "@rollup/plugin-commonjs";
import json from "@rollup/plugin-json";


export default {
    input: 'src/drift-zeromq.ts',
    output: {
        file: 'dist/drift-zeromq.js',
        format: 'es'
    },
    plugins: [
        commonjs({

        }),
        typescript({
            tsconfig: './tsconfig.json'
        }),
        json(),
        nodeResolve()
    ]
};