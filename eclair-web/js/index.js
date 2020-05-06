import("../crate/pkg/index.js").catch(console.error)
	.then(eclair => {
		window.eclair = eclair; // TODO: This is just for debugging to prove we're getting this in here
	});
