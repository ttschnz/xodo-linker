# My personal Workaround for Notions shitty PDF-Handling

Whenever I want to use a pdf in Notion and add a scribble or two to it, I notice, that it is really nerve wracking, how notion does not seem to care about how well they handle this kind of situation.

Intuitively one uploads a pdf to a page as a resource. When clicking on this resource, a browser window opens with the url of some AWS-CDN giving you a read-only version of the file. If you are not signed in, you'll need to do that first.
If you're lucky and the pdf opens in edge, you can make use of the only reason I have not removed it from my machine: Its powerful PDF-Editing tools.
If not, you are quite stuck.

That's why I thought about creating this workaround: To simply click on this link to open any PDF-Editing software with the pdf.
Personally, I've chosen Xodo, an amazing PDF-Editing engine. However I will try to do my best making this as configurable as possible, however layed out for Windows.

Have fun.

## using onedrive short-name

`fsutil file setshortname <PathName> <shortname>`
