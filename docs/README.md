# Quant·ion API documentation

A static React app. All content lives in `public/api/<api>.json`; `public/api/versions.json` lists every version and the API file it uses.

    npm install
    npm run dev
    npm run build

Every release is added to the version dropdown. Versions with the same API share one JSON file.

    npm run add-version -- 0.2.1              same API: points 0.2.1 at the current file
    npm run add-version -- 0.3.0 --new-api    API changed: copies the current file to 0.3.0.json to edit

Each version keeps its release date and commit message, shown on the Versions page. The message is the last commit's subject unless `--message "..."` is given. A version already listed without a date, such as 0.2.0, is marked as released:

    npm run add-version -- 0.2.0

Publishing a GitHub release builds the app and deploys it to GitHub Pages (`.github/workflows/docs.yml`).
