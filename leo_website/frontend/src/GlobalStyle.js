import { createGlobalStyle } from "styled-components";

export const GlobalStyle = createGlobalStyle`
    *,::after,::before {
        box-sizing: border-box;
        padding: 0;
        margin: 0;
    }
    
    body {
        background-color: #e0e0e0;
        display: flex;
        flex-direction: column;
        align-items: center;
        height: 100vh;
    }
`;