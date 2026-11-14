// The tests' «system» speaks Spanish, like the app before it followed the system (jsdom says
// en-US); a test sets another language through the mock backend.
Object.defineProperty(navigator, "languages", { value: ["es-CL", "es"], configurable: true });
